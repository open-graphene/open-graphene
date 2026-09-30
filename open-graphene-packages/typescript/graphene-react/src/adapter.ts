import { createContext, createElement, useContext, useEffect, useMemo, useSyncExternalStore, type ReactNode } from 'react';
import { queryOptions, useQuery, useQueryClient, useMutation, type UseQueryOptions, type UseQueryResult, type QueryKey } from '@tanstack/react-query';
import type { RpcMethod } from '@open-graphene/codec';
import { grapheneQueryKey } from './keys.js';
import { sharedLive, disabledLive, type LivePolicy, type LiveState, type LiveStream } from './live.js';
export interface SdkClient {
  readonly chainId: string;
  readonly rpc: { invoke<P, R>(method: RpcMethod<P, R>, params: P): Promise<R> };
  reconnect(): Promise<void>;
}
export type ReadOptions<T, D = T> = Pick<UseQueryOptions<T, Error, D, QueryKey>,
  'select' | 'staleTime' | 'gcTime' | 'refetchOnWindowFocus' | 'refetchOnReconnect' | 'refetchInterval'> & { readonly enabled?: boolean };
export type LiveReadOptions<T, D = T> = ReadOptions<T, D> & { readonly live?: boolean };
export type LiveQueryResult<T> = UseQueryResult<T, Error> & { readonly live: LiveState; readonly restartLive: () => void };
export interface ProviderProps<C> { readonly client: C; readonly scope?: string; readonly livePolicy?: LivePolicy; readonly children?: ReactNode }

export function createChainAdapter<C extends SdkClient>(chainId: string) {
  const Context = createContext<{ client: C; scope: string | undefined; policy: LivePolicy } | null>(null);
  function Provider({ client, scope, livePolicy, children }: ProviderProps<C>) {
    if (client.chainId !== chainId) throw new Error('React provider received a client for another chain');
    const maxRetries = livePolicy?.maxRetries ?? 3, delayMs = livePolicy?.delayMs ?? 250;
    const value = useMemo(() => ({ client, scope, policy: { maxRetries, delayMs } }), [client, scope, maxRetries, delayMs]);
    return createElement(Context.Provider, { value }, children);
  }
  function useConnection() {
    const value = useContext(Context);
    if (!value) throw new Error('Mount the matching Graphene chain provider first');
    return value;
  }
  function options<P, T>(client: C, name: string, params: P, fetch: (client: C, params: P) => Promise<T>, scope?: string) {
    if (client.chainId !== chainId) throw new Error('Query client belongs to another chain');
    return queryOptions<T, Error, T, QueryKey>({
      queryKey: grapheneQueryKey(client, name, params, scope),
      queryFn: () => fetch(client, params),
      retry: false as const,
      structuralSharing: false as const,
    });
  }
  function useRead<P, T, D = T>(name: string, params: P, fetch: (client: C, params: P) => Promise<T>, settings: LiveReadOptions<T, D> = {}, watch?: (client: C, params: P) => Promise<LiveStream<T>>): LiveQueryResult<D> {
    const { client, scope, policy } = useConnection();
    const cache = useQueryClient();
    const { live = false, ...querySettings } = settings;
    if (live && !watch) throw new Error('This query does not support live subscriptions');
    const base = options(client, name, params, fetch, scope);
    const keyId = JSON.stringify(base.queryKey);
    const entry = useMemo(() => live && watch
      ? sharedLive(cache, client, base.queryKey, () => watch(client, params), () => client.reconnect(), policy)
      : undefined, [cache, client, keyId, live, policy.maxRetries, policy.delayMs]);
    const active = live && settings.enabled !== false;
    useEffect(() => active ? entry?.acquire() : undefined, [entry, active]);
    const state = useSyncExternalStore(entry?.subscribe ?? noSubscribe, entry?.getSnapshot ?? getDisabled, getDisabled);
    const result = useQuery<T, Error, D, QueryKey>({
      queryKey: base.queryKey, queryFn: () => fetch(client, params),
      retry: false, structuralSharing: false,
      ...(live ? { staleTime: Infinity, refetchOnWindowFocus: false, refetchOnReconnect: false } : {}),
      ...querySettings,
    });
    return { ...result, live: active ? state : disabledLive, restartLive: entry?.restart ?? noop };
  }
  function rpcOptions<P, T>(client: C, method: RpcMethod<P, T>, params: P, scope?: string) {
    assertReadParams(params);
    // Encode first so defaults/omitted fields share keys and bigint never reaches TanStack's hash.
    const wire = method.encodeParams(params);
    return options(client, `${method.api}.${method.method}`, wire, c => c.rpc.invoke(method, params), scope);
  }
  function useRpc<P, T, D = T>(method: RpcMethod<P, T>, params: P, settings: ReadOptions<T, D> = {}) {
    assertReadParams(params);
    return useRead(`${method.api}.${method.method}`, method.encodeParams(params), c => c.rpc.invoke(method, params), settings);
  }
  /** Deliberate imperative work: no signer is ever passed as a mutation variable. */
  function useAction<V, R>(action: (client: C, value: V) => Promise<R>, invalidate = false) {
    const { client, scope } = useConnection();
    const cache = useQueryClient();
    return useMutation<R, Error, V>({
      mutationFn: value => action(client, value), retry: false, gcTime: 0,
      onSuccess: async () => {
        if (invalidate) await cache.invalidateQueries({ queryKey: grapheneQueryKey(client, '', null, scope).slice(0, 3) });
      },
    });
  }
  return { Provider, useClient: () => useConnection().client, options, useRead, rpcOptions, useRpc, useAction };
}
function assertReadParams(params: unknown): void {
  if (params && typeof params === 'object' && 'subscribe' in params && params.subscribe === true) {
    throw new Error('Use a live hook instead of subscribe:true in a cached RPC query');
  }
}
const noop = () => {};
const noSubscribe = (_listener: () => void) => noop;
const getDisabled = () => disabledLive;
