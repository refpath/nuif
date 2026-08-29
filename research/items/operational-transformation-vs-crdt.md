---
id: nuif:research:operational-transformation-vs-crdt
kind: synthesis
status: reviewed
title: Operational transformation (dOPT, GOT, Jupiter, Wave) versus CRDTs (Shapiro et al. 2011) - convergence conditions and hosting either above a canonical document
source:
  url: https://doi.org/10.1007/978-3-642-24550-3_29
  doi: 10.1007/978-3-642-24550-3_29
  authors: [Marc Shapiro, Nuno Preguiça, Carlos Baquero, Marek Zawirski, Clarence A. Ellis, Simon J. Gibbs, Chengzheng Sun, Xiaohua Jia, Yanchun Zhang, Yun Yang, David Chen, David A. Nichols, Pavel Curtis, Michael Dixon, John Lamping, Abdessamad Imine, Pascal Molli, Gérald Oster, Michaël Rusinowitch]
  published_at: "SSS 2011 (LNCS 6976, pp. 386-400) 2011-10; Ellis and Gibbs SIGMOD 1989; Sun et al. TOCHI 5(1) 1998-03; Nichols et al. UIST 1995; Imine et al. ECSCW 2003; Google Wave OT whitepaper 2009"
  license: Springer and ACM copyright with author preprints; Apache Wave whitepaper under Apache incubator terms
retrieved_at: 2026-08-29
tags: [operational-transformation, crdt, convergence, causality, intention-preservation, tp1, tp2, strong-eventual-consistency, collaboration, jupiter]
confidence: 0.9
claims: [nuif:claim:collab-profile]
relations:
  - type: extends
    target: nuif:research:automerge-yjs
    note: Supplies the formal definitions (SEC, CvRDT, CmRDT) that Automerge and Yjs instantiate.
  - type: related_to
    target: nuif:research:crdt-tree-move-operation
    note: The tree move CRDT is a CmRDT whose commutation is achieved by timestamp-ordered undo-do-redo.
  - type: related_to
    target: nuif:research:patch-theory-darcs-pijul
    note: Patch commutation and OT transformation both aim at order independence of concurrent edits.
  - type: compares_to
    target: nuif:research:figma
    note: Figma chose server-ordered last-writer-wins over both OT and CRDT.
  - type: supports
    target: nuif:claim:collab-profile
    note: Both families require replica metadata (state vectors, timestamps, tombstones) that a canonical document need not carry.
links:
  spec: [spec/10-collaboration-profile.md, spec/06-operations-and-patches.md]
  adr: [adrs/0005-collaboration-profile.md]
  rfc: []
  code: [crates/nuif-protocol]
  experiments: []
---

# Summary

Operational transformation (Ellis and Gibbs 1989) lets every site apply local operations immediately and transforms remote operations against concurrent ones before applying them; correctness depends on transformation functions satisfying TP1 (the two transformed orders yield the same state) and, for fully decentralised n-way concurrency, TP2 (transforming against equivalent sequences yields the same operation). Sun et al. (1998) state the CCI consistency model (convergence, causality preservation, intention preservation), define inclusion and exclusion transformations with a reversibility requirement, and give the GOT control algorithm with an undo/do/redo scheme. Imine et al. (2003) show with the SPIKE prover that the published transformation functions of Ellis-Gibbs violate TP1 and those of Ressel and Sun violate TP2 on three concurrent string operations. Jupiter (1995) and Google Wave avoid TP2 by a central server that serialises operations and transforms only against a single history, with one client operation in flight at a time. CRDTs (Shapiro et al. 2011) replace transformation with data types whose states form a monotonic join-semilattice (CvRDT) or whose concurrent operations commute under causal delivery (CmRDT); both satisfy strong eventual consistency by construction, and the two forms can emulate each other. A canonical document plus an operation log can host either family as a profile because both reduce, after their respective metadata is stripped, to a totally ordered sequence of semantic operations applied to a snapshot.

## Evidence

- Ellis and Gibbs, "Concurrency control in groupware systems", SIGMOD 1989, pp. 399-407, DOI 10.1145/67544.66963; abstract: users "can operate directly on the data without obtaining locks", the algorithm "must know some semantics of the operations", and desired behaviour "is non-serializable" (abstract via Semantic Scholar API, retrieved 2026-08-29; ACM page returned HTTP 403). The dOPT algorithm uses per-site state vectors and a transformation matrix indexed by operation type (Imine et al. 2003 §"Ellis's Transformation Functions" reproduces `Tii`, `Tid`, `Tdi` with priorities).
- Sun, Jia, Zhang, Yang, Chen, TOCHI 5(1):63-108, March 1998, DOI 10.1145/274444.274447: Definition 1 causal ordering; Definition 2 dependent and independent operations; Definition 3 intention as "the execution effect which can be achieved by applying O on the document state from which O was generated"; Definition 4 consistency model with convergence, causality preservation and intention preservation; Definition 6 total ordering for convergence (§4); Specification 1 `IT(Oa, Ob)` with precondition context-equivalence and Specification 2 `ET(Oa, Ob)`; Definition 9 reversibility `Oa = ET(IT(Oa, Ob), Ob)`; Functions LIT/LET; Algorithm 2 (GOT control algorithm); §7 integration with undo/do/redo. PDF https://www.cs.cityu.edu.hk/~jia/research/reduce98.pdf (retrieved 2026-08-29).
- Ressel, Nitsche-Ruhland, Gunzenhäuser, CSCW 1996, pp. 288-297, DOI 10.1145/240080.240305, introduce the adOPTed algorithm and the two transformation conditions later named TP1/TP2 (metadata via search; paper not retrieved).
- Imine, Molli, Oster, Rusinowitch, ECSCW 2003, DOI 10.1007/978-94-010-0068-0_15: conditions `C1: op1 ∘ T(op2, op1) ≡ op2 ∘ T(op1, op2)` and `C2: T(op3, op1 ∘ T(op2, op1)) = T(op3, op2 ∘ T(op1, op2))`; SPIKE finds a C1 counter-example for Ellis-Gibbs (Fig. 3) and C2 counter-examples for Ressel (Fig. 4) and Sun (Fig. 5) using concurrent `Ins(2,x)`, `Del(2)`, `Ins(3,y)`; only Suleiman's functions survive; counter-examples motivated tombstone transformation functions. PDF https://www.lri.fr/~mbl/ENS/CSCW/2013/papers/Imine-ECSCW03.pdf (retrieved 2026-08-29).
- Nichols, Curtis, Dixon, Lamping, "High-latency, low-bandwidth windowing in the Jupiter collaboration system", UIST 1995, DOI 10.1145/215585.215706 (metadata via Semantic Scholar API, retrieved 2026-08-29; PDF not retrieved).
- Google Wave OT whitepaper: the design is based on Jupiter; the server keeps "a single state space, which is the history of operations it has applied"; clients "wait for acknowledgement from the server before sending more operations" and compose pending operations; a streaming transformer processes two operations linearly; server serialisation removes the need for TP2. https://svn.apache.org/repos/asf/incubator/wave/whitepapers/operational-transform/operational-transform.html (retrieved 2026-08-29).
- Shapiro, Preguiça, Baquero, Zawirski, SSS 2011: Definition 3 strong eventual consistency; Definition 4 monotonic semilattice object; Theorem 1 "any state-based object that satisfies the monotonic semilattice property is SEC"; Definition 6 commutativity of updates; Theorem 2 op-based objects with commuting concurrent updates under causal delivery are SEC; §3.2 Theorems 3 and 4 (CmRDT and CvRDT emulation); §3.3 SEC is incomparable to sequential consistency. PDF https://gsd.di.uminho.pt/members/cbm/members/cbm/ps/sss2011.pdf (retrieved 2026-08-29); Springer landing page redirected to an authorisation endpoint.
- Tree CRDT as CmRDT: Kleppmann et al. prove `apply_ops_commutes` and SEC through the Gomes et al. framework (nuif:research:crdt-tree-move-operation, §4.2).
- Automerge merge rules (retrieved 2026-08-29, https://automerge.org/docs/reference/under-the-hood/merge-rules/): element IDs instead of indices, deterministic arbitrary ordering of concurrent inserts at one position, deterministic choice among concurrent map writes with conflicts retained.

## Mechanism

OT (Ellis-Gibbs shape, Sun et al. terminology):

```
local op o:      apply(o); broadcast(o, state_vector)
remote op o:     wait until causally ready (Sun Def. 5)
                 o' := transform o against every concurrent op already applied (IT), after
                       excluding operations not in o's context (ET) -- GOT, Algorithm 2
                 apply(o')
TP1 (C1):  apply(o1); apply(T(o2,o1))  ==  apply(o2); apply(T(o1,o2))
TP2 (C2):  T(o3, o1 ∘ T(o2,o1))  ==  T(o3, o2 ∘ T(o1,o2))
```

TP1 suffices when a server serialises operations and each client transforms only against the server's linear history (Jupiter, Wave). TP2 is needed for decentralised n-way concurrency and is where published functions fail (Imine et al.).

CRDT (Shapiro et al.):

```
CvRDT: states form a join-semilattice (S, ≤, ⊔); local update s' ≥ s; merge = s ⊔ s'
       convergence: ⊔ is commutative, associative, idempotent  ->  SEC (Theorem 1)
CmRDT: op = (prepare at source, effect at all replicas); reliable causal broadcast;
       concurrent effects commute (Def. 6)  ->  SEC (Theorem 2)
SEC:   eventual delivery + strong convergence (replicas that delivered the same updates have equal state) + termination
```

Hosting both above a canonical NUIF document (NUIF interpretation):

```
canonical snapshot S_0 (hash h_0)
profile log L = [op_1 ... op_n] in a total order chosen by the profile
  OT profile:   order = server sequence; op_i already transformed against L[1..i-1]; metadata = server seq no, client state vector
  CRDT profile: order = any causal linearisation; op_i carries replica id + Lamport/opId; metadata = ids, tombstones, clocks
materialise:  S_n = fold(apply, S_0, L)  ->  canonical hash h_n independent of profile metadata (spec/10 requirement)
```

Both profiles require that every operation be semantic (identity-addressed, precondition-guarded) so that transformation or commutation is defined per operation type; index-addressed operations force string-style transformation functions with the TP2 hazards above.

## NUIF relevance

**Borrow**
- Sun et al.'s three-part consistency model (convergence, causality preservation, intention preservation) as the stated requirements of spec/10, with intention preservation mapped to preservation of each operation's preconditions.
- Shapiro et al.'s SEC as the convergence guarantee the profile demands of any engine, with Theorem 2's causal-delivery assumption made explicit as a transport requirement.
- The Jupiter/Wave server-serialised design as the reference architecture for a centralised NUIF profile, because it needs only TP1.

**Adapt**
- Transformation functions for NUIF must be defined per semantic operation pair (move/move, move/remove, set/set, insert/insert under one parent) rather than per index arithmetic; the CRDT tree move paper already supplies the move/move and move/cycle cases.
- Causal identifiers and clocks are profile data (spec/10); the canonical patch format keeps only base revision plus ordered operations, and checkpoints strip tombstones.

**Reject**
- Decentralised OT with n-way concurrency as a NUIF profile, because TP2-correct transformation functions are demonstrably hard to obtain (Imine et al.) and offer no advantage over CmRDTs for identity-addressed trees.
- Mandating one CRDT library (Automerge, Yjs) in the specification, consistent with spec/10.

## Open questions

- Whether intention preservation has a precise formulation for tree operations with preconditions, or whether it should be replaced by "preconditions of every applied operation held at application time" plus explicit conflict objects.
- Whether an OT profile and a CRDT profile can share one wire operation schema, or whether OT's transformed operations (context-dependent) must be re-expressed as identity-addressed operations before they enter the canonical log.
- Verification strategy: the tree move CRDT has Isabelle proofs; a NUIF operation set would need an analogous mechanised commutation proof for property and relation operations.
