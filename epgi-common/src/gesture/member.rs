// use epgi_core::foundation::InlinableDwsizeVec;

// use super::PointerEvent;

// pub const DEFAULT_CONFIDENCE: f32 = 0.5;
// pub const WINNING_CONFIDENCE: f32 = 1.0;
// pub const LOSING_CONFIDENCE: f32 = 0.0;

// pub trait GestureRecognizerTeamMember {
//     fn handle_pointer_event(&mut self, event: PointerEvent) -> f32;
//     fn handle_victory(&mut self);
//     // We using a moving Box Self to ensure whoever notifies defeat will also withdraw us from their record.
//     // Also, boxed trait object will produce an extra virtual destructor call if we use it by reference.
//     fn handle_defeat(self: Box<Self>);
// }

// pub struct GestureRecognizerCompetingTeam {
//     policy: GestureRecognizerTeamPolicy,
//     self_has_won: bool,
//     member_has_won: bool,
//     members: InlinableDwsizeVec<(Box<dyn GestureRecognizerTeamMember>, f32)>,
// }

// #[derive(PartialEq, Eq, Clone, Copy, Debug)]
// pub enum GestureRecognizerTeamPolicy {
//     Competing,
//     Cooperative,
//     Hereditary,
// }

// impl GestureRecognizerCompetingTeam {
//     pub fn new(
//         policy: GestureRecognizerTeamPolicy,
//         members: impl IntoIterator<Item = Box<dyn GestureRecognizerTeamMember>>,
//     ) -> Self {
//         Self {
//             policy,
//             self_has_won: false,
//             member_has_won: false,
//             members: members
//                 .into_iter()
//                 .map(|member| (member, DEFAULT_CONFIDENCE))
//                 .collect(),
//         }
//     }

//     pub fn push(&mut self, member: impl GestureRecognizerTeamMember + 'static) {
//         debug_assert!(
//             !self.self_has_won && !self.member_has_won,
//             "You should not push new members into a team after it has won"
//         );
//         self.members
//             .push((Box::new(member) as _, DEFAULT_CONFIDENCE))
//     }

//     pub fn push_front(&mut self, member: impl GestureRecognizerTeamMember + 'static) {
//         debug_assert!(
//             !self.self_has_won && !self.member_has_won,
//             "You should not push new members into a team after it has won"
//         );
//         self.members
//             .insert(0, (Box::new(member) as _, DEFAULT_CONFIDENCE))
//     }
// }

// impl GestureRecognizerTeamMember for GestureRecognizerCompetingTeam {
//     fn handle_pointer_event(&mut self, event: PointerEvent) -> f32 {
//         if self.members.is_empty() {
//             return 0.0;
//         }
//         let mut highest_confidence = f32::MIN;
//         // THIS SHOULD USE Vec::extract_if https://github.com/rust-lang/rust/issues/43244
//         // Yes, they spent 6 years on this very basic API design
//         self.members = self
//             .members
//             .drain(..)
//             .filter_map(|(mut member, _)| {
//                 let confidence = member.handle_pointer_event(event.clone());
//                 highest_confidence = f32::max(highest_confidence, confidence);
//                 if confidence <= LOSING_CONFIDENCE {
//                     member.handle_defeat();
//                     None
//                 } else {
//                     Some((member, confidence))
//                 }
//             })
//             .collect();

//         // Competing teams may defer choosing a winning member even when itself was declared victory
//         // So we has to check for winning conditions when handling events
//         use GestureRecognizerTeamPolicy::*;
//         if self.policy == Competing && self.self_has_won && !self.member_has_won {
//             if self.members.len() == 1 || highest_confidence >= WINNING_CONFIDENCE {
//                 self.handle_victory();
//             }
//         }

//         return highest_confidence;
//     }

//     fn handle_victory(&mut self) {
//         self.self_has_won = true;
//         let member_count = self.members.len();
//         if member_count == 0 {
//             panic!(
//                 "This gesture recognizer should have requested withdrawal already, \
//                 but it is still notified as the winner."
//             )
//         }
//         if member_count == 1 {
//             // Even competing team has to declare victory for the sole survivor
//             return self.members[0].0.handle_victory();
//         }

//         let mut winner_index = None;
//         use GestureRecognizerTeamPolicy::*;
//         if self.policy == Competing || self.policy == Cooperative {
//             let (highest_member_index, (_, highest_confidence)) = self
//                 .members
//                 .iter()
//                 .enumerate()
//                 // Rev ensures we get the first candidate in `max_by` if multiple candidate report the same confidence
//                 .rev()
//                 .max_by(|(_, (_, confidence_1)), (_, (_, confidence_2))| {
//                     confidence_1
//                         .partial_cmp(confidence_2)
//                         .expect("Confidence should not be NaN")
//                 })
//                 .expect("Impossible to fail");
//             if self.policy == Cooperative || *highest_confidence >= WINNING_CONFIDENCE {
//                 winner_index = Some(highest_member_index);
//             }
//         } else {
//             winner_index = Some(0);
//         }

//         if let Some(winner_index) = winner_index {
//             // We only mess with initial order if we are certain we have found a winner
//             let (mut winner, winner_confidence) = self.members.swap_remove(winner_index);
//             winner.handle_victory();
//             self.member_has_won = true;
//             let losers = std::mem::replace(&mut self.members, [(winner, winner_confidence)].into());
//             for (loser, _) in losers.into_iter() {
//                 loser.handle_defeat();
//             }
//         }
//     }

//     fn handle_defeat(mut self: Box<Self>) {
//         for (member, _) in self.members.drain(..) {
//             member.handle_defeat()
//         }
//     }
// }
