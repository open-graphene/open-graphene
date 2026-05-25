# SDK Builder Bridge Ergonomics Spike

## Reader and action

This document records whether the current Swaplock and BitShares SDK builder bridge repetition should be abstracted now.

After reading it, a maintainer should know whether to add a macro/helper for `*OperationBuilderTypes` and per-operation `*ChainTypes` impls, or keep the repetition explicit until more pressure exists.

## Current bridge shape

The cross-chain SDK builder seam is:

```text
common input -> generic builder -> chain generated-type bridge -> generated Transaction
```

The shared crate `open-graphene-sdk-operations` owns:

```text
GrapheneOperationBuilderTypes
TransferChainTypes
AccountCreateChainTypes
AssetIssueChainTypes
AssetCreateChainTypes
build_*_transaction_for
```

Each chain binding crate owns one zero-sized generated-type bridge marker:

```text
SwaplockOperationBuilderTypes
BitSharesOperationBuilderTypes
```

Each chain then implements the common base trait plus the operation-specific traits against its generated protocol types.

## Measurement

A normalized comparison replaced chain-specific spelling with a generic chain token:

```text
Swaplock -> Chain
swaplock -> chain
BitShares -> Chain
bitshares -> chain
```

Measured files:

```text
operation_builder_types.rs
transfer.rs
account_create.rs
asset_issue.rs
asset_create.rs
```

Result:

```text
operation_builder_types.rs: lines 39/39, normalized_equal=true, similarity=1.000, differing_lines~=0
transfer.rs: lines 270/270, normalized_equal=true, similarity=1.000, differing_lines~=0
account_create.rs: lines 458/458, normalized_equal=true, similarity=1.000, differing_lines~=0
asset_issue.rs: lines 327/327, normalized_equal=true, similarity=1.000, differing_lines~=0
asset_create.rs: lines 498/498, normalized_equal=true, similarity=1.000, differing_lines~=0
```

Trait impl sizes in one chain:

```text
GrapheneOperationBuilderTypes: 27 nonblank lines
TransferChainTypes: 22 nonblank lines
AccountCreateChainTypes: 41 nonblank lines
AssetIssueChainTypes: 22 nonblank lines
AssetCreateChainTypes: 59 nonblank lines
```

The repetition is real. It is also currently straightforward and reviewable.

## Abstraction options considered

### Option 1: keep explicit bridge impls now

Keep the duplicated impls in each chain package.

Pros:

- The generated-type boundary remains visible in ordinary Rust code.
- Chain-specific differences can appear naturally without fighting a macro.
- Tests already prove equivalence across current Swaplock and BitShares flows.
- No new macro API or generator policy is introduced.

Cons:

- A third chain would repeat roughly the same base and operation-specific bridge code.
- Adding a fifth operation requires two similar chain impls.

### Option 2: add a macro for chain bridge impls

A macro could emit the base and per-operation trait impls from a mapping of generated type names.

Pros:

- It would reduce visible LOC in chain packages.
- It would make a third chain cheaper if the generated names keep matching.

Cons:

- It would hide the exact generated type mapping behind macro parameters.
- Compile errors would point through macro expansion.
- The macro would become a second mini-generator for SDK bridge code.
- If the third chain differs, the macro either grows options or becomes leaky.

### Option 3: generator-emitted bridge impls

The bindings generator could emit bridge impls for a curated common-chain SDK profile.

Pros:

- It matches the observed identical pattern across two chains.
- It would reduce manual chain boilerplate the most.

Cons:

- This crosses into generator-emitted SDK surface earlier than necessary.
- It risks freezing the bridge API before a third chain or fifth flow proves the seam.
- It makes SDK policy harder to review separately from protocol binding generation.

## Recommendation

Do not abstract the builder bridge yet.

The repeated code is perfectly duplicated after chain-name normalization, but there are only two active chains and four flows. The current explicit impls are useful documentation of the generated-type boundary. A macro or generator would reduce LOC, but it would also make the protocol-to-SDK seam less obvious and would likely become a small generator before the project has enough pressure to justify it.

Keep the current bridge impls explicit until at least one of these triggers happens:

1. A third Graphene chain needs the same SDK flows.
2. A fifth operation flow repeats the same bridge pattern across both chains.
3. A real chain-specific divergence appears and clarifies what should remain configurable.
4. Compile-time or review friction from the explicit impls becomes measurable.

When one of those triggers happens, prefer a narrow macro or generator profile that only emits bridge impl boilerplate. It must not emit SDK input models, RPC orchestration, signing, fee lookup, broadcast policy, or operation JSON semantics.

## Next practical step

The best next engineering step is not builder bridge abstraction. Better candidates are:

1. Add a fifth SDK flow to create more operation-shape pressure.
2. Run a live BitShares proof if chain id, account, and funds are available.
3. Spike a narrow shared generated protocol primitive only if a concrete adapter friction point appears.
