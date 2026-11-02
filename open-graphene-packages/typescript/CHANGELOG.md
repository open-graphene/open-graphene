# Changelog

## 0.1.0 — release candidate, unpublished

- Native TypeScript SDKs for Actanet, BitShares and R-Squared, generated from the
  shared protocol specification, for Node.js and browser applications.
- Generated JSON/RPC bindings for 95/78/64 operation variants and 49/32/31 RPC
  methods, plus FC encoding and typed factories for all 88/71/59 nonvirtual
  operations (Actanet/BitShares/R-Squared respectively). Actanet includes data
  rooms, content cards and fee grants (operations 80 to 94).
- One chain API source shared by the three chains: transaction preparation, fees,
  signing, broadcast with callbacks, inclusion tracking and inspection, and
  restoration of saved signed transactions; wallet, memo, multisignature and
  confidential-operation helpers.
- Signatures follow the node rule `fc::ecc::public_key::is_canonical` on every
  chain; transactions are signed for the chain ID of the connected network.
- Separate `chain-actanet-react`, `chain-bitshares-react` and `chain-rsquared-react`
  packages, shared `react-core`, TanStack Query reads, live subscriptions,
  transaction mutations, shared-connection providers and generated preparation
  hooks for every nonvirtual operation.
- React 18.3.1 and 19.3.0 compatibility checks, independent packed-package
  consumers, complete package licenses and dependency-ordered release artifacts.

Live RPC availability depends on node permissions. Generated coverage and local
tests do not prove successful live execution of every protocol operation. npm
publication and registry consumer validation are pending. See the
[release guide](../../docs/TYPESCRIPT-RELEASE.md).
