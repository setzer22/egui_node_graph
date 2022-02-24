# CHANGELOG

## 0.2.0

### Changed
- Under the `persistence` feature `serde::Serialize`/`Deserialize` is now
  derived for `GraphEditorState` and all its relevant types.
- `NodeTemplateIter` now requires the list of templates returned by user code to
  be owned. This circumvents several issues that came with having a trait return
  an iterator of references.
- Generic parameters in `NodeDataTrait` are now associated types instead. This
  makes implementing the types possible in more situationns.

### Added
- New `CreatedNode` response by @jorgeja
