# CHANGELOG

## 0.4.0

### Changed
- Make the node finder search bar case insensitive. By @matthijsjanssens
- Add a `UserState` parameter to `build_node`. By @setzer22
- Updated to egui 0.19. By @MathiasPius
- Separate `UserState` from `GraphState` to allow more flexible borrowing patterns. By @kkngsm
- Expose `UserState` and the `node_id` to `value_widget` for more flexible widgets. By @setzer22

### Added
- Connections snap to available ports. By @kkngsm
- Add box selection and multi-node movement. By @setzer22
- Nodes can decide whether to show the delete button. By @huisedenanhai

### Bugfixes
- Disconnect responses are now return before node removal responses. By @bpostlethwaite

## 0.3.0

### Changed
- Update to egui 0.18. By @Veykril and @gmorenz
- Make editor work inside egui windows or panels. By @Imberflur
- Remove macros from the example in favor of simpler lambdas. By @philpax
- Connections are now drawn with the same color as the datatype. By @kkngsm
- The name method in `DataType` now returns a `Cow<str>` instead of `&str`. By @setzer22
- The `DeleteNode` response is now split into `DeleteNodeUi` and
  `DeleteNodeFull`, the latter including all the data for the deleted node. By @setzer22

### Added
- CI setup. By @fenollp
- Draw connections using bézier curves. By @gmorenz
- Add UserResponse to WidgetValueTrait, allowing widgets to execute side effects. By @IsseW
- Send disconnect events on node delete. By @setzer22
- Light mode support. By @kkngsm
- Allow configurable node titlebar colors. By @kkngsm
- Add more information to the various Response types (`DisconnectEvent`, `ConnectEventEnded`). By @setzer22

## Bugfixes
- Fix panning when clicking outside the editor area. By @fkaa
- Fix node finder sometimes drawing before other elements. By @fkaa
- Fix visual glitch where the X and the node title would sometimes slightly overlap. By @setzer22


## 0.2.0

### Changed
- Under the `persistence` feature `serde::Serialize`/`Deserialize` is now
  derived for `GraphEditorState` and all its relevant types. By @setzer22
- `NodeTemplateIter` now requires the list of templates returned by user code to
  be owned. This circumvents several issues that came with having a trait return
  an iterator of references. By @setzer22
- Generic parameters in `NodeDataTrait` are now associated types instead. This
  makes implementing the types possible in more situationns. By @setzer22

### Added
- New `CreatedNode` response. By @jorgeja
