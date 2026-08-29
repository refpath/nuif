# 06 — Operations, patches and merge

Status: draft.

Operations are stable, serializable semantic mutations. Core operations: create entity, delete entity, move/reorder entity, set/unset property, add/remove relation, set extension, bind/unbind token, apply instance override and transaction.

A patch declares a base snapshot/content identity and ordered operations. Preconditions MAY guard expected prior values.

Undo is represented as inverse semantic operations or transaction history; it is not part of canonical document state.

Three-way merge MUST prefer stable identity. Implementations MUST surface typed conflicts rather than selecting arbitrary winners for semantic conflicts.
