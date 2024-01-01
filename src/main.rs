use bevy::{
    prelude::*,
    DefaultPlugins,
};
use bevy_particle_systems::{*, VelocityModifier::*,
};
use bevy_egui::{egui::{self, Widget, ImageButton}, EguiContexts, EguiPlugin};


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
        10 * (1.04_f64).powf(self.total_fingers as f64) as u64
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

#[derive(Component)]
struct BurstTimer(Timer);

impl Default for BurstTimer {
    fn default() -> Self {
        BurstTimer(Timer::from_seconds(0.1, TimerMode::Once))
    }
}

fn collect_score_system(
    mut score: ResMut<Score>,
    mut clicker_events: EventReader<ClicksEmitted>,
    mut available_particle_systems: Query<(Entity, &mut BurstTimer), Without<Playing>>,
    mut commands: Commands,
) {
    for ClicksEmitted(clicks) in clicker_events.read() {
        score.stored_clicks += clicks;
        for (_, (entity, mut timer)) in (0..*clicks).zip(available_particle_systems.iter_mut()) {
            commands.entity(entity).insert(Playing);
            timer.0.reset();
        }
    }
}

fn burst_deactivator_system(
    mut commands: Commands,
    mut burst_timers: Query<(Entity, &mut BurstTimer), With<Playing>>,
    time: Res<Time>,
) {
    for (entity, mut burst_timer) in burst_timers.iter_mut() {
        if burst_timer.0.tick(time.delta()).just_finished() {
            commands.entity(entity).remove::<Playing>();
        }
    }
    
}

struct IconCache {
    finger: egui::TextureId,
    atlas: TextureAtlas,
}

impl IconCache {
    fn uv_rect_for(&self, idx: usize) -> egui::Rect {
        let px_rect = self.atlas.textures.get(idx).unwrap();
        let tex_rect = self.atlas.size;
        egui::Rect {
            min: egui::Pos2::new(px_rect.min.x / tex_rect.x, px_rect.min.y / tex_rect.y),
            max: egui::Pos2::new(px_rect.max.x / tex_rect.x, px_rect.max.y / tex_rect.y),
        }
    }
}

fn ui_system(
    mut hands: Query<(&mut HandState, &mut TillCanClickTimer, &Children, Entity), Without<ClickerState>>,
    mut all_clickers: Query<(&ClickerState, &mut TillCanClickTimer), With<ClickerLabel>>,
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut clicker_events: EventWriter<ClicksEmitted>,
    mut icons: Local<Option<IconCache>>,
    asset_server: Res<AssetServer>
) {
    if icons.is_none() {
        let image = asset_server.load("finger.png");
        let finger = contexts.add_image(image.clone());
        let atlas = TextureAtlas::from_grid(image, Vec2::new(32.0, 32.0), 4, 1, None, None);
        *icons = Some(IconCache { finger, atlas });
    }

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
                        
                        egui::Grid::new("fingers").num_columns(5).striped(true).show(ui, |ui| {
                            for (idx, clicker) in Iterator::enumerate(clickers.iter()) {
                                // end row every 5
                                if idx % 5 == 0 && idx != 0 {
                                    ui.end_row();
                                }

                                let flip_idx = if idx % 5 == 0 {
                                    2
                                } else {
                                    0
                                };
                                let (state, mut timer) = all_clickers.get_mut(*clicker).unwrap();
                                if timer.0.finished() {
                                    if ImageButton::new(egui::widgets::Image::new(egui::load::SizedTexture::new(
                                        icons.as_ref().unwrap().finger,
                                        [32.0,32.0]
                                    )).uv(icons.as_ref().unwrap().uv_rect_for(flip_idx + 0))).ui(ui).clicked() {
                                        timer.0.reset();
                                        clicker_events.send(ClicksEmitted(state.per_click))
                                    }
                                } else {
                                    ImageButton::new(egui::widgets::Image::new(egui::load::SizedTexture::new(
                                        icons.as_ref().unwrap().finger,
                                        [32.0,32.0]
                                    )).uv(icons.as_ref().unwrap().uv_rect_for(flip_idx + 1))).selected(true).ui(ui);
                                }
                            }
                        });
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
                            clap_timer.0.reset();
                            clicker_events.send(ClicksEmitted(clickers.len() as u64));
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    // set up backdrop
    commands.spawn(SpriteBundle {
        texture: asset_server.load("backdrop.png").into(),
        transform: Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(4.0)),
        ..SpriteBundle::default()
    });

    commands.spawn(Hand::default()).with_children(|parent| {
        parent.spawn(Clicker::default());
    });

    for y_idx in -4..4 {
        for x_idx in -6..7 {
            commands
                .spawn(ParticleSystemBundle {
                    particle_system: ParticleSystem {
                        max_particles: 5_000,
                        texture: asset_server.load("spark.png").into(),
                        spawn_rate_per_second: 1000.0.into(),
                        initial_speed: JitteredValue::jittered(200.0, -50.0..50.0),
                        velocity_modifiers: vec![Drag(0.01.into())],
                        lifetime: JitteredValue::jittered(1.0, -0.5..0.5),
                        color: ColorOverTime::Gradient(Curve::new(vec![
                            CurvePoint::new(Color::RED, 0.0),
                            CurvePoint::new(Color::YELLOW, 0.75),
                            CurvePoint::new(Color::rgba(1.0, 1.0, 1.0, 0.0), 1.0),
                        ])),
                        looping: true,
                        system_duration_seconds: 10.0,
                        max_distance: Some(300.0),
                        scale: 0.5.into(),
                        ..ParticleSystem::default()
                    },
                    transform: Transform::from_xyz(100.0 * x_idx as f32, 100.0 * y_idx as f32, 0.0),
                    ..ParticleSystemBundle::default()
            }).insert(BurstTimer::default());
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(
            ImagePlugin::default_nearest(),
        ))
        .add_plugins(EguiPlugin)
        .add_plugins(ParticleSystemPlugin::default())
        .add_event::<ClicksEmitted>()
        .insert_resource(Score::default())
        .add_systems(Update, (ui_system, update_timers_system, collect_score_system, burst_deactivator_system))
        .add_systems(Startup, setup)
        .run();
}
