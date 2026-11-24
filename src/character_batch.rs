//! Shared atomic commands for the character timeline, menus and setup guide.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

impl Editor {
    pub(super) fn attach_rigged_layers(
        &mut self,
        rows: &[usize],
        track: u64,
        matrix: Matrix,
    ) -> bool {
        if !matrix.iter().all(|value| value.is_finite())
            || !self.doc.layers.iter().any(|layer| {
                layer
                    .options
                    .character_piece
                    .as_ref()
                    .is_some_and(|parent| parent.track == track && parent.eye_parent.is_none())
            })
        {
            return false;
        }
        let rows: BTreeSet<_> = rows
            .iter()
            .copied()
            .filter(|&row| {
                self.doc
                    .layers
                    .get(row)
                    .and_then(|layer| layer.options.character_piece.as_ref())
                    .is_some_and(|piece| {
                        piece.track != track
                            && (piece.eye_parent.is_some() || piece.face != FacePart::None)
                            && (piece.eye_parent != Some(track) || piece.matrix != matrix)
                    })
            })
            .collect();
        if rows.is_empty() {
            return false;
        }
        let masks: BTreeSet<_> = self
            .doc
            .layers
            .iter()
            .enumerate()
            .filter_map(|(row, layer)| {
                layer
                    .options
                    .character_piece
                    .as_ref()
                    .map(|piece| (row, piece))
            })
            .filter(|(row, piece)| {
                piece.face == FacePart::Eye
                    && (piece.eye_parent == Some(track) || rows.contains(row))
            })
            .map(|(_, piece)| piece.track)
            .collect();
        self.finish_pending_pen();
        self.checkpoint();
        for row in rows {
            let piece = self.doc.layers[row]
                .options
                .character_piece
                .as_mut()
                .unwrap();
            piece.eye_parent = Some(track);
            piece.matrix = matrix;
            if piece.eye_mask.is_some_and(|mask| !masks.contains(&mask)) {
                piece.eye_mask = None;
            }
        }
        self.recover();
        true
    }

    pub(super) fn mirror_rigged_layers(&mut self, rows: &[usize], opposite: bool) -> bool {
        let rows: BTreeSet<_> = rows
            .iter()
            .copied()
            .filter(|&row| {
                self.doc
                    .layers
                    .get(row)
                    .and_then(|layer| layer.options.character_piece.as_ref())
                    .is_some_and(|piece| piece.eye_parent.is_some())
            })
            .collect();
        if rows.is_empty() || self.doc.layers.len().saturating_add(rows.len()) > 100 {
            return false;
        }
        let sources: Vec<_> = rows
            .iter()
            .map(|&row| {
                let piece = self.doc.layers[row]
                    .options
                    .character_piece
                    .as_ref()
                    .unwrap();
                (row, piece.track, piece.eye_mask)
            })
            .collect();
        self.finish_pending_pen();
        self.checkpoint();
        let first = self.doc.layers.len();
        let mut tracks = BTreeMap::new();
        for &(row, old, _) in &sources {
            if let Some(new) = self.mirror_face_layer_inner(row, opposite, None) {
                tracks.insert(old, new);
            }
        }
        for (layer, (_, _, mask)) in self.doc.layers[first..].iter_mut().zip(sources) {
            if let Some(piece) = layer.options.character_piece.as_mut() {
                piece.eye_mask = mask.map(|mask| tracks.get(&mask).copied().unwrap_or(mask));
            }
        }
        self.recover();
        true
    }

    pub(super) fn rename_rigged_layers(&mut self, names: &[(usize, String)]) -> bool {
        let names: BTreeMap<_, _> = names
            .iter()
            .filter_map(|(row, name)| {
                let name: String = name.trim().chars().take(128).collect();
                self.doc
                    .layers
                    .get(*row)
                    .filter(|layer| !name.is_empty() && layer.name != name)
                    .map(|_| (*row, name))
            })
            .collect();
        if names.is_empty() {
            return false;
        }
        self.finish_pending_pen();
        self.checkpoint();
        for (row, name) in names {
            let renamed_bone = self.doc.layers[row]
                .options
                .character_piece
                .as_ref()
                .filter(|piece| piece.eye_parent.is_none())
                .and_then(|piece| {
                    self.feature
                        .symbol_stack
                        .last()
                        .and_then(|(parent, _, _, _)| {
                            let mut bones = parent
                                .extras
                                .rig
                                .bones
                                .iter()
                                .filter(|bone| bone.targets.contains(&piece.track));
                            let first = bones.next()?;
                            bones.next().is_none().then_some(first.id)
                        })
                });
            let layer = &mut self.doc.layers[row];
            layer.name = name;
            if let Some(piece) = layer.options.character_piece.as_mut()
                && renamed_bone.is_some()
            {
                piece.renamed_bone = renamed_bone;
            }
        }
        self.recover();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn editor() -> (Editor, Vec<(u64, String)>, usize, usize) {
        let ctx = egui::Context::default();
        let mut editor = Editor::new(&eframe::CreationContext::_new_kittest(ctx));
        editor.doc = crate::fresh_document();
        editor.recovery = std::env::temp_dir().join(format!(
            "character-batch-{}-{:?}.json",
            std::process::id(),
            std::thread::current().id()
        ));
        editor.doc.layers[0].frames[0].push(Mark {
            points: vec![[20., 30.], [120., 140.]].into(),
            rectangle: true,
            fill: Some([100, 120, 140, 255]),
            ..Default::default()
        });
        editor.canvas.selected = vec![(0, 0)];
        editor.convert_symbol_kind(false);
        let mut second = editor.doc.layers[0].frames[0][0].clone();
        second.style.track = crate::features::id();
        second.style.instance.as_mut().unwrap().matrix[4] += 200.;
        editor.doc.layers[0].frames[0].push(second);
        editor.canvas.selected = vec![(0, 0), (0, 1)];
        editor.create_character_from_selection();
        let targets = editor.rigged_layer_targets();
        assert_eq!(targets.len(), 2);
        editor.add_rigged_piece(FacePart::Eye, targets[0].0, None);
        editor.add_rigged_piece(FacePart::Pupil, targets[0].0, None);
        let eye = editor
            .doc
            .layers
            .iter()
            .position(|layer| {
                layer
                    .options
                    .character_piece
                    .as_ref()
                    .is_some_and(|piece| piece.face == FacePart::Eye)
            })
            .unwrap();
        let pupil = editor
            .doc
            .layers
            .iter()
            .position(|layer| {
                layer
                    .options
                    .character_piece
                    .as_ref()
                    .is_some_and(|piece| piece.face == FacePart::Pupil)
            })
            .unwrap();
        let mask = editor.doc.layers[eye]
            .options
            .character_piece
            .as_ref()
            .unwrap()
            .track;
        editor.doc.layers[pupil]
            .options
            .character_piece
            .as_mut()
            .unwrap()
            .eye_mask = Some(mask);
        (editor, targets, eye, pupil)
    }
    #[test]
    fn batch_attach_preserves_selected_eye_mask_and_undoes_once() {
        let (mut editor, targets, eye, pupil) = editor();
        let before = editor.undo.len();
        let mask = editor.doc.layers[pupil]
            .options
            .character_piece
            .as_ref()
            .unwrap()
            .eye_mask;
        assert!(editor.attach_rigged_layers(&[eye, pupil], targets[1].0, rig::IDENTITY));
        assert_eq!(editor.undo.len(), before + 1);
        for row in [eye, pupil] {
            assert_eq!(
                editor.doc.layers[row]
                    .options
                    .character_piece
                    .as_ref()
                    .unwrap()
                    .eye_parent,
                Some(targets[1].0)
            );
        }
        assert_eq!(
            editor.doc.layers[pupil]
                .options
                .character_piece
                .as_ref()
                .unwrap()
                .eye_mask,
            mask
        );
        editor.history(false);
        assert_eq!(
            editor.doc.layers[pupil]
                .options
                .character_piece
                .as_ref()
                .unwrap()
                .eye_parent,
            Some(targets[0].0)
        );
    }
    #[test]
    fn batch_mirror_remaps_masks_and_batch_names_each_use_one_undo() {
        let (mut editor, _, eye, pupil) = editor();
        let first = editor.doc.layers.len();
        let before = editor.undo.len();
        assert!(editor.mirror_rigged_layers(&[eye, pupil], true));
        assert_eq!(editor.doc.layers.len(), first + 2);
        assert_eq!(editor.undo.len(), before + 1);
        let new_eye = editor.doc.layers[first..]
            .iter()
            .find_map(|layer| {
                layer
                    .options
                    .character_piece
                    .as_ref()
                    .filter(|piece| piece.face == FacePart::Eye)
                    .map(|piece| piece.track)
            })
            .unwrap();
        let new_pupil = editor.doc.layers[first..]
            .iter()
            .find_map(|layer| {
                layer
                    .options
                    .character_piece
                    .as_ref()
                    .filter(|piece| piece.face == FacePart::Pupil)
            })
            .unwrap();
        assert_eq!(new_pupil.eye_mask, Some(new_eye));
        editor.history(false);
        assert_eq!(editor.doc.layers.len(), first);
        let before = editor.undo.len();
        assert!(editor.rename_rigged_layers(&[(eye, "L_Eye".into()), (pupil, "L_Pupil".into())]));
        assert_eq!(editor.undo.len(), before + 1);
        assert_eq!(editor.doc.layers[eye].name, "L_Eye");
        editor.history(false);
        assert_ne!(editor.doc.layers[eye].name, "L_Eye");
    }
}
