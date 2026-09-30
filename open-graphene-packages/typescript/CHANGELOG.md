# Changelog

## 0.1.0 — release candidate, unpublished

- Native TypeScript SDKs for Swaplock and BitShares, generated from the shared
  protocol specification, for Node.js and browser applications.
- Generated JSON/RPC bindings for 95/78 operation variants and 47/32 RPC methods,
  plus FC encoding and typed factories for all 88/71 nonvirtual operations
  (Swaplock/BitShares respectively).
- Transaction preparation, fees, signing, broadcast and inclusion tracking;
  wallet, memo, multisignature and confidential-operation helpers.
- Separate `chain-swaplock-react` and `chain-bitshares-react` packages, shared
  `react-core`, TanStack Query reads, live subscriptions, transaction mutations
  and generated preparation hooks for all 159 nonvirtual operations.
- React 18.3.1 and 19.3.0 compatibility checks, independent packed-package
  consumers, complete package licenses and dependency-ordered release artifacts.

Live RPC availability depends on node permissions. Generated coverage and local
tests do not prove successful live execution of every protocol operation. The new
React room transaction check still needs a successful testnet run; npm publication
and registry consumer validation are also pending. See the
[release guide](../../docs/TYPESCRIPT-RELEASE.md).
