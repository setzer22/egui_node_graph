
use crate::{color_hex_utils::*, NodeTemplateIter, NodeTemplateTrait, NodeTrait};

use egui::*;

#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeFinder {
    pub query: String,
    /// Reset every frame. When set, the node finder will be moved at that position
    pub position: Option<Pos2>,
    pub just_spawned: bool,
}

impl NodeFinder {
    pub fn new_at(pos: Pos2) -> Self {
        NodeFinder {
            query: "".into(),
            position: Some(pos),
            just_spawned: true,
        }
    }

    /// Shows the node selector panel with a search bar. Returns whether a node
    /// archetype was selected and, in that case, the finder should be hidden on
    /// the next frame.
    pub fn show<NodeTemplate: NodeTemplateTrait>(
        &mut self,
        ui: &mut Ui,
        all_kinds: impl NodeTemplateIter<Item = NodeTemplate>,
    ) -> Option<NodeTemplate> {
        let background_color;
        let text_color;

        if ui.visuals().dark_mode {
            background_color = color_from_hex("#3f3f3f").unwrap();
            text_color = color_from_hex("#fefefe").unwrap();
        } else {
            background_color = color_from_hex("#fefefe").unwrap();
            text_color = color_from_hex("#3f3f3f").unwrap();
        }

        ui.visuals_mut().widgets.noninteractive.fg_stroke = Stroke::new(2.0, text_color);

        let frame = Frame::dark_canvas(ui.style())
            .fill(background_color)
            .inner_margin(vec2(5.0, 5.0));

        // The archetype that will be returned.
        let mut submitted_archetype = None;
        frame.show(ui, |ui| {
            ui.vertical(|ui| {
                let resp = ui.text_edit_singleline(&mut self.query);
                if self.just_spawned {
                    resp.request_focus();
                    self.just_spawned = false;
                }

                let mut query_submit = resp.lost_focus() && ui.input().key_down(Key::Enter);

                Frame::default()
                    .inner_margin(vec2(10.0, 10.0))
                    .show(ui, |ui| {
                        for kind in all_kinds.all_kinds() {
                            let kind_name = kind.node_finder_label().to_string();
                            if kind_name
                                .to_lowercase()
                                .contains(self.query.to_lowercase().as_str())
                            {
                                if ui.selectable_label(false, kind_name).clicked() {
                                    submitted_archetype = Some(kind);
                                } else if query_submit {
                                    submitted_archetype = Some(kind);
                                    query_submit = false;
                                }
                            }
                        }
                    });
            });
        });

        submitted_archetype
    }
}
