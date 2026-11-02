import {
  createContext,
  createElement,
  useContext,
  type ReactNode,
} from 'react';
import type { SharedConnection } from '@open-graphene/transport';

export interface ConnectionContext {
  /** Supplies a host-owned connection to every consumer below it. It never closes it. */
  ConnectionProvider(props: {
    connection: SharedConnection;
    children: ReactNode;
  }): ReactNode;
  /** The connection of the nearest provider, to pass as `connection` in SDK options. */
  useConnection(): SharedConnection;
}

/** A provider and hook for one chain's shared connection, named after `chain` in errors. */
export function createConnectionContext(chain: string): ConnectionContext {
  const Context = createContext<SharedConnection | undefined>(undefined);
  const providerName = `${chain}ConnectionProvider`;

  return {
    ConnectionProvider(props) {
      return createElement(Context.Provider, {
        value: props.connection,
        children: props.children,
      });
    },
    useConnection() {
      const connection = useContext(Context);
      if (!connection) {
        throw new Error(`${providerName} is required.`);
      }
      return connection;
    },
  };
}
