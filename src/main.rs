use bevy::prelude::*;
use bevy_egui::{egui::{self, Widget}, EguiContexts, EguiPlugin};

#[derive(Component, Default)]
enum HandState {
    #[default]
    Filling,
    Combined,
    Autoed,
}

#[derive(Component, Default)]
struct HandLabel;

#[derive(Bundle, Default)]
struct Hand {
    label: HandLabel,
    state: HandState,
    clap_timer: TillCanClickTimer,
}

#[derive(Component, Default)]
struct ClickerLabel;

#[derive(Component)]
struct TillCanClickTimer(Timer);

impl Default for TillCanClickTimer {
    fn default() -> Self {
        TillCanClickTimer(Timer::from_seconds(1.0, TimerMode::Once))
    }
}

#[derive(Component)]
struct ClickerState {
    per_click: u64,
}

impl Default for ClickerState {
    fn default() -> Self {
        ClickerState { per_click: 1 }
    }
}

#[derive(Bundle, Default)]
struct Clicker {
    label: ClickerLabel,
    till_can_click: TillCanClickTimer,
    state: ClickerState,
}

#[derive(Resource, Default)]
struct Score {
    stored_clicks: u64,
    total_fingers: u64,
    total_hands: u64,
}

#[derive(Event)]
struct ClicksEmitted(u64);

impl Score {
    fn finger_cost(&self) -> u64 {
        10 //self.total_fingers * 10 + 10
    }

    fn hand_cost(&self) -> u64 {
        self.total_hands * 10 + 10
    }

    fn combine_cost(&self) -> u64 {
        30
    }

    fn auto_cost(&self) -> u64 {
        60
    }
}

fn collect_score_system(
    mut score: ResMut<Score>,
    mut clicker_events: EventReader<ClicksEmitted>,
) {
    for ClicksEmitted(clicks) in clicker_events.iter() {
        score.stored_clicks += clicks;
    }
}

fn ui_system(
    mut hands: Query<(&mut HandState, &mut TillCanClickTimer, &Children, Entity), Without<ClickerState>>,
    mut all_clickers: Query<(&ClickerState, &mut TillCanClickTimer), With<ClickerLabel>>,
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut clicker_events: EventWriter<ClicksEmitted>,
) {
    for (mut hand, mut clap_timer, clickers, hand_entity) in hands.iter_mut() {
        egui::Window::new("Hand")
            .id(egui::Id::new(hand_entity))
            .show(contexts.ctx_mut(), |ui| {

                match *hand {
                    HandState::Filling => {
                        // buy finger
                        if score.stored_clicks >= score.finger_cost() {
                            if ui.button(format!("Buy Finger ({})", score.finger_cost())).clicked() {
                                commands.spawn(Clicker::default()).set_parent(hand_entity);
                                score.stored_clicks -= score.finger_cost();
                                score.total_fingers += 1;
                            }
                        } else {
                            ui.label(format!("Buy finger ({})", score.finger_cost()));
                        }
                        // make hand
                        if score.stored_clicks >= score.combine_cost() {
                            if ui.button(format!("Combine Hand ({})", score.combine_cost())).clicked() {
                                *hand = HandState::Combined;
                                score.stored_clicks -= score.combine_cost();
                                score.total_hands += 1;
                            }
                        } else {
                            ui.label(format!("Combine Hand ({})", score.combine_cost()));
                        }
                        
                        for clicker in clickers {
                            let (state, mut timer) = all_clickers.get_mut(*clicker).unwrap();
                            if timer.0.finished() {
                                if ui.button("Click").clicked() {
                                    timer.0.reset();
                                    clicker_events.send(ClicksEmitted(state.per_click))
                                }
                            } else {
                                egui::ProgressBar::new(timer.0.percent()).desired_width(100.0).ui(ui);
                            }
                        }
                    }
                    
                    HandState::Combined => {
                        // make hand auto
                        if score.stored_clicks >= score.auto_cost() {
                            if ui.button(format!("Make Auto ({})", score.auto_cost())).clicked() {
                                *hand = HandState::Autoed;
                                score.stored_clicks -= score.auto_cost();
                            }
                        } else {
                            ui.label(format!("Make Auto ({})", score.auto_cost()));
                        }

                        if clap_timer.0.finished() {
                            if ui.button("Clap").clicked() {
                                clap_timer.0.reset();
                                clicker_events.send(ClicksEmitted(clickers.len() as u64));
                            }
                        } else {
                            egui::ProgressBar::new(clap_timer.0.percent()).desired_width(100.0).ui(ui);
                        }
                    }

                    HandState::Autoed => {
                        if clap_timer.0.finished() {
                            score.stored_clicks += clickers.len() as u64;
                            clap_timer.0.reset();
                        }
                        
                        egui::ProgressBar::new(clap_timer.0.percent()).desired_width(100.0).ui(ui);

                    }
                }

                
            });
    }

    egui::Window::new("Store").show(contexts.ctx_mut(), |ui| {
        ui.label(format!("Clicks: {}", score.stored_clicks));
        // buy hand
        if score.stored_clicks >= score.hand_cost() {
            if ui.button(format!("Buy Hand ({})", score.hand_cost())).clicked() {
                // spawn with empty children so our query can find it
                commands.spawn(Hand::default()).with_children(|_parent| {});
                score.stored_clicks -= score.hand_cost();
                score.total_hands += 1;
            }
        } else {
            ui.label(format!("Buy Hand ({})", score.hand_cost()));
        }
    });

}

fn update_timers_system(mut all_clickers: Query<&mut TillCanClickTimer>, time: Res<Time>) {
    for mut timer in &mut all_clickers.iter_mut() {
        timer.0.tick(time.delta());
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Hand::default()).with_children(|parent| {
        parent.spawn(Clicker::default());
        parent.spawn(Clicker::default());
    });
}
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_event::<ClicksEmitted>()
        .insert_resource(Score::default())
        .add_systems(Update, (ui_system, update_timers_system, collect_score_system))
        .add_systems(Startup, setup)
        .run();
}
