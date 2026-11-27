//! Quick copy shares immutable symbol/media resources but creates new instance IDs.
use crate::{Editor, Mark};
pub(crate) type Fragments = std::collections::HashMap<(usize, usize), (Vec<Mark>, Vec<Mark>)>;
impl Editor {
    pub(crate) fn can_quick_copy(&mut self, ids: &[(usize, usize)]) -> bool {
        let bindings = ids
            .iter()
            .filter_map(|&(l, m)| self.doc.layers[l].frames[self.frame].get(m))
            .filter(|m| {
                self.doc
                    .extras
                    .rig
                    .bindings
                    .iter()
                    .any(|b| b.target == m.style.track)
            })
            .count();
        if self.doc.extras.rig.bindings.len() + bindings > 512 {
            self.status = crate::i18n::tr("Cannot duplicate more rigged instances.").into();
            false
        } else {
            true
        }
    }
    pub(crate) fn quick_copy_selection(
        &mut self,
        ids: &[(usize, usize)],
        partial: &Fragments,
    ) -> Vec<((usize, usize), Mark)> {
        let mut prepared = Vec::new();
        for &(l, m) in ids {
            if self.doc.layers[l].options.locked || !self.doc.layers[l].visible {
                continue;
            }
            if let Some((_, inside)) = partial.get(&(l, m)) {
                prepared.extend(inside.iter().cloned().map(|m| (l, m)));
            } else if let Some(mark) = self.doc.layers[l].frames[self.frame].get(m) {
                prepared.push((l, mark.clone()));
            }
        }
        let mut boundaries = std::collections::HashMap::new();
        let mut result = Vec::new();
        for (l, mut mark) in prepared {
            let old = mark.style.track;
            let track = crate::features::id();
            if let Some(mut binding) = self
                .doc
                .extras
                .rig
                .bindings
                .iter()
                .find(|b| b.target == old)
                .cloned()
            {
                binding.target = track;
                self.doc.extras.rig.bindings.push(binding);
                for bone in &mut self.doc.extras.rig.bones {
                    if bone.targets.contains(&old) {
                        bone.targets.push(track);
                    }
                }
            }
            mark.style.track = track;
            crate::linked_fill::remap_ids(&mut mark, &mut |id| {
                *boundaries.entry(id).or_insert_with(crate::features::id)
            });
            let frame = &mut self.doc.layers[l].frames[self.frame];
            let index = frame.len();
            frame.push(mark.clone());
            result.push(((l, index), mark));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quick_copy_keeps_symbol_resources_and_rebinds_instance_identity() {
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(
            eframe::egui::Context::default(),
        ));
        e.doc = crate::Document::default();
        e.layer = 0;
        e.frame = 0;
        let mut mark = Mark {
            points: vec![[10., 10.], [100., 100.]].into(),
            rectangle: true,
            ..Default::default()
        };
        mark.style.track = 123;
        mark.style.instance = Some(crate::features::Instance {
            clock: Default::default(),
            symbol: 44,
            matrix: crate::rig::IDENTITY,
            mode: Default::default(),
            first: 0,
            born: 0,
        });
        e.doc.layers[0].frames[0] = vec![mark.clone()].into();
        e.doc.extras.rig.bindings.push(crate::rig::Binding {
            target: 123,
            symbol: 44,
            matrix: crate::rig::IDENTITY,
            bounds: [10., 10., 100., 100.],
            cells: Default::default(),
            pins: Default::default(),
        });
        let copy = e.quick_copy_selection(&[(0, 0)], &Default::default());
        assert_eq!(copy.len(), 1);
        assert_eq!(copy[0].1.style.instance.as_ref().unwrap().symbol, 44);
        assert_eq!(e.doc.extras.rig.bindings.len(), 2);
        assert_ne!(copy[0].1.style.track, 123);
        assert_eq!(e.doc.extras.rig.bindings[1].target, copy[0].1.style.track);
        assert_eq!(e.doc.layers[0].frames[0][0].style.track, 123);
    }
    #[test]
    fn copying_a_partial_selection_does_not_cut_the_source() {
        let mut e = Editor::new(&eframe::CreationContext::_new_kittest(
            eframe::egui::Context::default(),
        ));
        e.doc = crate::Document::default();
        e.layer = 0;
        e.frame = 0;
        let source = Mark {
            points: vec![[0., 0.], [100., 100.]].into(),
            rectangle: true,
            ..Default::default()
        };
        let mut fragment = source.clone();
        fragment.points = vec![[0., 0.], [50., 100.]].into();
        e.doc.layers[0].frames[0] = vec![source.clone()].into();
        let partial = std::collections::HashMap::from([((0, 0), (vec![], vec![fragment.clone()]))]);
        let copies = e.quick_copy_selection(&[(0, 0)], &partial);
        assert_eq!(e.doc.layers[0].frames[0][0].points, source.points);
        assert_eq!(copies[0].1.points, fragment.points);
        assert_eq!(e.doc.layers[0].frames[0].len(), 2);
    }
}
