---
id: nuif:research:lenses-foster-boomerang
kind: paper
status: reviewed
title: Lens laws from Foster et al. (TOPLAS 2007) and Boomerang (POPL 2008) to symmetric, edit and delta lenses
source:
  url: https://www.cis.upenn.edu/~bcpierce/papers/lenses-toplas-final.pdf
  doi: 10.1145/1232420.1232424
  authors: [J. Nathan Foster, Michael B. Greenwald, Jonathan T. Moore, Benjamin C. Pierce, Alan Schmitt, Aaron Bohannon, Alexandre Pilkiewicz, Martin Hofmann, Daniel Wagner, Zinovy Diskin, Yingfei Xiong, Krzysztof Czarnecki]
  published_at: 2007-05
  license: ACM (TOPLAS 29(3) Article 17; POPL 2008 pp. 407-419; POPL 2011 pp. 371-384; POPL 2012 pp. 495-508); JOT 10 (2011) 6:1-25 (open access); author preprints retrieved
retrieved_at: 2026-08-29
tags: [bidirectional-transformations, lenses, well-behaved, symmetric-lenses, edit-lenses, delta-lenses, view-update, synchronization]
confidence: 0.93
claims: [nuif:claim:sync-not-regenerate]
relations:
  - type: related_to
    target: nuif:research:symmetric-lenses
    note: Definition 2.1 (PutRL, PutLR) and the embedding of asymmetric lenses (Definition 9.1, Theorem 9.4) are summarised here with locators.
  - type: related_to
    target: nuif:research:retentive-lenses
    note: Retentive lenses strengthen the state-based laws with region preservation; edit and delta lenses reach similar guarantees by making the edit explicit.
  - type: related_to
    target: nuif:research:bidirectional-evaluation-direct-manipulation
    note: Sketch-n-Sketch's Theorems 3.5/3.6 are the GetPut/PutGet analogues; its user lenses satisfy no laws.
  - type: related_to
    target: nuif:research:structured-merge
    note: Delta lenses require deltas to compose (PutPut on deltas), which is what a structured three-way merge must respect.
  - type: supports
    target: nuif:research:retentive-lenses
links:
  spec: [spec/06-operations-and-patches.md, spec/09-provenance-and-fidelity.md]
  adr: []
  rfc: [rfcs/0003-authored-resolved-provenance.md]
  code: [crates/nuif-protocol]
  experiments: [nuif:experiment:v0-responsive-card]
---

# Summary

Foster et al. define a lens as a pair of partial functions get (`l↗ : C ⇀ A`) and putback (`l↘ : A × C ⇀ C`) and call it well-behaved when GetPut (`l↘(l↗ c, c) ⊑ c`) and PutGet (`l↗(l↘(a, c)) ⊑ a`) hold; the optional PutPut law (`l↘(a′, l↘(a, c)) ⊑ l↘(a′, c)`) defines very well-behaved lenses and is deliberately not required because map, flatten, merge and conditionals fail it. Boomerang restates lenses for strings with a total `create` and laws GetPut, PutGet, CreateGet, and introduces dictionary (resourceful) lenses that align chunks by key rather than position; the corresponding weakening of obliviousness is the EquivPut law (quasi-obliviousness). Hofmann, Pierce and Wagner's symmetric lenses replace get/put by `putr` and `putl` over a complement `C` with laws PutRL and PutLR, prove that asymmetric lenses embed and that every symmetric lens factors as two asymmetric lenses back to back, and identify alignment as an explicit non-goal. Edit lenses make edits first-class: a module is a set with a monoid of edits acting partially on it, an edit lens translates edits through a complement while preserving a consistency relation, and Theorem 7.1 gives a one-to-one correspondence with state-based symmetric lenses under the overwrite monoid. Diskin, Xiong and Czarnecki's delta lenses take model spaces as categories whose arrows are deltas; a well-behaved delta lens satisfies incidence laws (GetInc, PutInc1, PutInc2), identity laws (GetId, PutId) and PutGet, while very well-behaved additionally satisfies GetGet and PutPut on deltas; Theorem 5 recovers a well-behaved state-based lens from a delta lens plus a differencing function and Theorem 6 recovers PutPut only conditionally on the differencing being composition-compatible. NUIF interpretation follows.

## Evidence

Foster, Greenwald, Moore, Pierce, Schmitt, "Combinators for bidirectional tree transformations", ACM TOPLAS 29(3), Article 17, May 2007 (DOI 10.1145/1232420.1232424 verified via Crossref 2026-08-29; author preprint from cis.upenn.edu retrieved 2026-08-29, preprint page numbers cited):

- Definition 3.1 (Lenses): partial get `l↗ : V ⇀ V` and partial putback `l↘ : V × V ⇀ V`. Source: §3, p. 6.
- Definition 3.2 (Well-behaved lenses): `l ∈ C ⇌ A` iff `l↗(C) ⊆ A`, `l↘(A × C) ⊆ C`, GetPut `l↘(l↗ c, c) ⊑ c`, PutGet `l↗(l↘(a, c)) ⊑ a`, where `f(x) ⊑ y` means `f(x)` is undefined or equals `y`. Source: §3, p. 6.
- PutPut `l↘(a′, l↘(a, c)) ⊑ l↘(a′, c)` is "optional"; a well-behaved lens also satisfying it is very well behaved. Source: §3, p. 7.
- "we will not require PutPut because some of our lens combinators-in particular, map, flatten, merge, and conditionals-fail to satisfy it for reasons that seem pragmatically unavoidable." Source: §3, p. 7; map counterexample §5 p. 22 (modifying a child differs from deleting and re-adding it); flatten counterexample §9 pp. 47-48.
- Definition 3.3 (Totality): `l ∈ C ⇐⇒ A` if `C ⊆ dom(l↗)` and `A × C ⊆ dom(l↘)`; footnote 3 notes well-behavedness "is rather trivial in the absence of totality". Source: §3, p. 7.
- Definition 3.7 (Oblivious): `l↘(a, c) = l↘(a, c′)` for all `a, c, c′`; Lemma 3.9: a total oblivious lens has bijective get; conversely every bijection induces a total oblivious lens; §11 notes every oblivious lens is very well behaved. Source: §3, p. 8; §11, p. 59.
- Ω ("missing") as the argument to putback when no concrete view exists; conventions `l↗Ω = Ω`, `l↘(Ω, c) = Ω`; Lemma 3.20. Source: §3 "Dealing with Creation", p. 11.
- Composition `(l; k)↘(a, c) = l↘(k↘(a, l↗ c), c)`; Lemma 4.3 (well-behavedness preserved), Lemma 4.4 (totality preserved). Source: §4, p. 13.
- Combinators: id, compose, const, hoist, plunge, fork/xfork, filter, prune, add, focus, rename, map, wmap, copy, merge, ccond/acond/cond, list combinators (hd, tl, list_map, rotate, group, concat, list_filter), flatten, pivot, join. Source: §§4-9.
- Foundations: well-behaved lenses correspond to Gottlob-Paolini-Zicari dynamic views and very well behaved lenses to Bancilhon-Spyratos constant-complement translators; footnote 9: with total components the laws including PutPut "characterize the set C as isomorphic to A × B for some B". Source: §10, pp. 50-51.
- The paper explicitly rejects choosing a minimal translation by an ordering (Johnson-Rosebrugh-Dampney) in favour of the programmer specifying the update policy with the view definition; Buneman-Khanna-Tan intractability of inferring minimal view updates is cited. Source: §10, p. 51.
- Framing is state-based, not trace-based: "we are interested here in the final tree a′, not the particular sequence of edit operations". Source: §2, footnote 1, p. 5.

Bohannon, Foster, Pierce, Pilkiewicz, Schmitt, "Boomerang: Resourceful Lenses for String Data", POPL 2008, pp. 407-419 (DOI 10.1145/1328438.1328487 verified via DBLP 2026-08-29; author preprint retrieved):

- Basic lens: `get ∈ C → A`, `put ∈ A → C → C`, `create ∈ A → C` with laws `put (get c) c = c` (GetPut), `get (put a c) = a` (PutGet), `get (create a) = a` (CreateGet); total components; laws are part of the definition. Source: §1, footnote 1.
- Positional Kleene-star put "mangles" reordered output, "a show-stopper for many of the applications". Source: §1, §2.
- Dictionary lens: components get, parse (concrete to skeleton plus dictionary), key, create, put; `key E` and `match ⟨l⟩` combinators; Theorem 3.1: a dictionary lens coerces to a basic lens satisfying the basic laws. Source: §3.
- Quasi-obliviousness: `c ∼ c′ ⟹ put a c = put a c′` (EquivPut) for an equivalence `∼` on C; every dictionary lens is quasi-oblivious with respect to key-respecting chunk reordering; oblivious iff the maximal equivalence is total; very well behaved iff constant complement. Source: §4.
- "Very well behavedness is a strong condition and imposing it on all lenses would prevent writing many useful transformations"; the alternative "is disallowing deletions". Source: §4.
- Typing: unambiguous concatenation and unambiguous iteration of regular languages, decidable (Fact 2.1); required for well-definedness and well-behavedness (`lambig` counterexample). Source: §2.

Hofmann, Pierce, Wagner, "Symmetric Lenses", POPL 2011, pp. 371-384 (DOI 10.1145/1926385.1926428 verified via DBLP; author preprint retrieved):

- Definition 2.1: `ℓ ∈ X ↔ Y` has complement `C`, `missing ∈ C`, `putr ∈ X × C → Y × C`, `putl ∈ Y × C → X × C`, with PutRL `putr(x, c) = (y, c′) ⟹ putl(y, c′) = (x, c′)` and PutLR symmetric. Source: §2.
- Symmetric PutPut variants "appear too strong to be desirable in practice". Source: §2.
- Definition 3.2 (lens equivalence) via a relation on complements; needed because associativity of composition and other laws hold only up to equivalence. Source: §3.
- Definition 4.2 (composition, `C = k.C × ℓ.C`); symmetric lenses form a category with equivalence classes as arrows; Theorem 5.1: no categorical products; tensor product is symmetric monoidal. Source: §§4-5.
- Definition 9.1: asymmetric lens `ℓ` embeds as `ℓ^sym` with complement `{f ∈ Y → X | ∀y. get(f(y)) = y}` and `missing = create`; Theorem 9.4: every symmetric lens factors as `(k1^sym)^op ; k2^sym`. Source: §9.
- "One important non-goal of the present paper is dealing with the (critical) issue of alignment"; deltas or edit monoids suggested as future work. Source: §2, §11.

Hofmann, Pierce, Wagner, "Edit Lenses", POPL 2012, pp. 495-508 (DOI 10.1145/2103656.2103715 verified via Crossref; author preprint from dmwit.com retrieved):

- Motivation: prior lenses "only consider edits of the form 'overwrite the whole structure'"; complements hold small alignment information. Source: abstract, §1-2, Figure 1.
- Definition 3.2 (monoid action, partial), Definition 3.3 (module `⟨X, init_X, ∂X, ⊙_X⟩`), Definition 3.4 (stateful monoid homomorphism). Source: §3.
- Definition 3.5 (symmetric edit lens): complement `C`, `init ∈ C`, homomorphisms `⇛ : ∂X × C → ∂Y × C` and `⇚`, consistency relation `K ⊆ X × C × Y` with `(init_X, init, init_Y) ∈ K` and preservation: if `(x, c, y) ∈ K`, `dx x` defined and `⇛(dx, c) = (dy, c′)`, then `dy y` is defined and `(dx x, c′, dy y) ∈ K` (and symmetrically). Source: §3.
- Theorem 3.7 (totality on consistent states); Definition 3.8 and Theorem 3.9 (equivalence via bisimulation). Source: §3.
- List module generators `mod(p, dx)`, `ins(i)`, `del(i)`, `reorder(f)`, `fail`; mapping lens carries insertions, deletions and reorderings across unchanged; container lenses (Theorem 5.7, `T(ℓ)` functorial). Source: §§4-5.
- Theorem 7.1: with the overwrite monoid, `|−|` and `∂` give a one-to-one correspondence between equivalence classes of edit lenses and state-based symmetric lenses. Source: §7.

Diskin, Xiong, Czarnecki, "From State- to Delta-Based Bidirectional Model Transformations: the Asymmetric Case", Journal of Object Technology 10 (2011) 6:1-25 (DOI 10.5381/jot.2011.10.1.a6 printed; retrieved 2026-08-29):

- Definition 1 restates state-based well-behaved lenses (GetPut, PutGet) and very well-behaved (PutPut). Source: §2.1.
- Two failures of state-based lenses: composed lenses using different alignment keys turn a rename into delete plus insert (P1, §2.2); PutPut fails for "a quite reasonable transformation" because differencing, not propagation, is non-compositional (P2, §2.3, §3.1 equations (3)-(4)).
- Definition 3 (model space: a connected category whose arrows are deltas); Definition 4 (delta lens `(A, B, get, put)` with `get` a graph morphism and `put : B₁ × A₀ → A₁`). Source: §4.1-4.2.
- Laws (Figure 9): GetInc, PutInc1 (`put(b, A)` defined iff `A.get₀ = source(b)`), PutInc2 (`source(put(b, A)) = A`); GetId, PutId (`id_A = put(id_B, A)`); PutGet (`get₁(put(b, A)) = b`); GetGet, PutPut (`put(b; b′, A) = put(b, A); put(b′, A′)`). Well-behaved = incidence + identity + PutGet; very well-behaved adds GetGet and PutPut. GetPut is not required on deltas; PutId is described as the identity-preservation content of GetPut. Source: §4.2.
- Theorem 2 and Theorem 3: composition preserves (very) well-behavedness; delta lenses form a category. Source: §4.3.
- Definition 6 (differencing with DifInc, DifId), Theorem 5 (every well-behaved delta lens plus differencing yields a well-behaved state-based lens), Theorem 6 (very well-behaved yields conditional PutPut when `dif(B, B″) = dif(B, B′); dif(B′, B″)`); a leap-day example shows PutPut can still fail on deltas. Source: §4.4.

## Mechanism

State-based asymmetric lens (TOPLAS 2007, Definitions 3.1-3.3):

```
get   l↗ : C ⇀ A
put   l↘ : A × C ⇀ C
GetPut  l↘(l↗ c, c) ⊑ c
PutGet  l↗(l↘(a, c)) ⊑ a
PutPut  l↘(a′, l↘(a, c)) ⊑ l↘(a′, c)          -- optional; "very well behaved"
total   C ⊆ dom(l↗) ∧ A × C ⊆ dom(l↘)
oblivious  l↘(a, c) = l↘(a, c′)   ⟹ get bijective (Lemma 3.9), very well behaved
```

Boomerang basic and dictionary lenses (POPL 2008 §1, §3-4):

```
get : C → A ; put : A → C → C ; create : A → C
GetPut   put (get c) c = c
PutGet   get (put a c) = a
CreateGet get (create a) = a
EquivPut c ∼ c′ ⟹ put a c = put a c′        -- quasi-oblivious w.r.t. key-preserving reorderings
```

Symmetric lens (POPL 2011, Definition 2.1):

```
ℓ : X ↔ Y = (C, missing ∈ C, putr : X×C → Y×C, putl : Y×C → X×C)
PutRL  putr(x, c) = (y, c′) ⟹ putl(y, c′) = (x, c′)
PutLR  putl(y, c) = (x, c′) ⟹ putr(x, c′) = (y, c′)
embedding of asymmetric ℓ: C = {f : Y → X | get ∘ f = id}, missing = create
factorisation: every symmetric lens = (k1^sym)^op ; k2^sym
```

Edit lens (POPL 2012, Definitions 3.3, 3.5):

```
module  ⟨X, init_X, ∂X, ⊙⟩ : monoid ∂X acting partially on X
lens    (C, init, ⇛ : ∂X×C → ∂Y×C, ⇚ : ∂Y×C → ∂X×C, K ⊆ X×C×Y)
        ⇛, ⇚ stateful monoid homomorphisms (identity ↦ identity, composition threaded through C)
        (init_X, init, init_Y) ∈ K
        (x, c, y) ∈ K ∧ dx x defined ∧ ⇛(dx, c) = (dy, c′) ⟹ dy y defined ∧ (dx x, c′, dy y) ∈ K
```

Delta lens (JOT 2011, Definition 4, Figure 9):

```
model spaces A, B : categories, arrows = deltas
get : A → B graph morphism (get₀ on models, get₁ on deltas) ; put : B₁ × A₀ → A₁
GetInc, PutInc1, PutInc2   incidence (put applies only to the matching base model)
GetId   get₁(id_A) = id_B ;  PutId  put(id_B, A) = id_A
PutGet  get₁(put(b, A)) = b
GetGet  get₁(a; a′) = get₁(a); get₁(a′) ;  PutPut  put(b; b′, A) = put(b, A); put(b′, A′)
well-behaved = incidence + identity + PutGet ; very well-behaved = + GetGet + PutPut
```

## NUIF relevance

- **Borrow**: NUIF's patch model (ordered operations with base snapshot identity and preconditions) is a delta lens `put`: PutInc1/PutInc2 are the base-snapshot precondition and the requirement that the resulting source patch applies to exactly that base; PutId is the requirement that a no-op design edit yields an empty source patch; PutGet is the requirement that re-lowering the patched source reproduces the design delta. These three, not the state-based laws, are the laws NUIF should require of a `lossless` adapter.
- **Borrow**: Foster et al.'s explicit refusal to define "minimal" updates by an ordering (TOPLAS §10) and Boomerang's argument that very well-behavedness would disallow deletions justify NUIF requiring well-behaved (PutId, PutGet, incidence) plus retentiveness (nuif:research:retentive-lenses) rather than PutPut; NUIF's "minimal source patch" should be defined as retention of unaffected source regions, which edit lenses achieve by translating `ins/del/reorder/mod` edits rather than by minimising a metric.
- **Borrow**: Symmetric lenses justify NUIF's system-level symmetry: source code and design each hold private data (comments, formatting, tokens, layout intent), so the complement `C` is the correspondence record; Theorem 9.4 shows this can still be implemented as two directional adapters back to back.
- **Adapt**: Boomerang's key-based alignment (dictionary lenses, EquivPut) becomes NUIF's stable entity identity: adapters must align by identity and fall back to structural matching only when identity is absent, which is exactly the quasi-oblivious regime.
- **Adapt**: Theorem 5/6 of the delta-lens paper give the conformance test for adapters that expose only state: compute deltas by differencing, check PutId and PutGet, and test PutPut only for delta pairs whose differencing composes.
- **Reject**: Requiring PutPut (very well-behaved) of NUIF adapters; the TOPLAS combinators map, merge and conditionals - all needed for component instantiation and conditional layout - fail it for pragmatic reasons.
- **Reject**: Purely state-based (overwrite) synchronisation in the protocol; Theorem 7.1 of Edit Lenses shows the state-based view is recoverable from edit lenses, but the converse loses alignment (P1/P2 in the delta-lens paper), which is the failure mode "regenerate instead of sync" that nuif:claim:sync-not-regenerate names.

## Open questions

- Which subset of NUIF operations forms a monoid with a partial action in the edit-lens sense, given that `create/delete/move` and `set property` do not commute and transactions group them?
- Retentive lenses are stated for state-based lenses; is there a delta-lens formulation of retentiveness that NUIF can adopt directly for patch replay?
- Boomerang's unambiguity typing has no analogue for tree-structured source; can tree-sitter grammars (nuif:research:tree-sitter) provide the equivalent guarantee that a correspondence record identifies a unique source span?
- The delta-lens laws assume a single base model per put; NUIF three-way merge with concurrent design and source edits needs a multi-source generalisation (Diskin's later multiary delta lenses were not reviewed here).
