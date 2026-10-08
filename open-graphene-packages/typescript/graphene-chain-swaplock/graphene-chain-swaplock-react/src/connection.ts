import {
  createContext,
  createElement,
  useContext,
  type ReactNode,
} from 'react';
import type { SharedConnection } from '@open-graphene/transport';

const ConnectionContext = createContext<SharedConnection | undefined>(
  undefined,
);

/** Supplies the host-owned connection to every Swaplock consumer below it. */
export function SwaplockConnectionProvider(props: {
  connection: SharedConnection;
  children: ReactNode;
}) {
  return createElement(ConnectionContext.Provider, {
    value: props.connection,
    children: props.children,
  });
}

export function useSwaplockConnection(): SharedConnection {
  const connection = useContext(ConnectionContext);
  if (!connection) {
    throw new Error('SwaplockConnectionProvider is required.');
  }
  return connection;
}
