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

#[derive(Resource)]
struct Score {
    stored_clicks: u64,
    total_fingers: u64,
    total_hands: u64,
    buildings: u32,
}

impl Default for Score {
    fn default() -> Self {
        Score {
            stored_clicks: 0,
            total_fingers: 1,
            total_hands: 0,
            buildings: 1,
        }
    }
}

#[derive(Event)]
struct ClicksEmitted(u64);

const MULTIPLIER_TABLE : [u64; 10] = [
    40, 80, 100, 150, 200, 250, 300, 350, 375, 400
];

const CASHOUT_TABLE : [u64; 3] = [
    10000, 100000000, 50000000000
];

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

    fn multiplier(&self) -> u64 {
        let mut multiplier = 1u64;
        for lmt in MULTIPLIER_TABLE.iter() {
            if self.total_fingers >= *lmt {
                multiplier *= 2;
            }
        };
        // prestige bonus
        multiplier *= 10u64.pow(self.buildings - 1);
        multiplier
    }

    fn next_multiplier(&self) -> Option<u64> {
        for lmt in MULTIPLIER_TABLE.iter() {
            if self.total_fingers < *lmt {
                return Some(*lmt);
            }
        };
        None
    }

    fn cashout_cost(&self) -> Option<u64> {
        if (self.buildings as usize) < CASHOUT_TABLE.iter().count() {
            Some(CASHOUT_TABLE[self.buildings as usize - 1])
        } else {
            None
        }
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
                                        clicker_events.send(ClicksEmitted(state.per_click * score.multiplier()))
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
                                clicker_events.send(ClicksEmitted((clickers.len() as u64) * score.multiplier()));
                            }
                        } else {
                            egui::ProgressBar::new(clap_timer.0.percent()).desired_width(100.0).ui(ui);
                        }
                    }

                    HandState::Autoed => {
                        if clap_timer.0.finished() {
                            clap_timer.0.reset();
                            clicker_events.send(ClicksEmitted((clickers.len() as u64) * score.multiplier()));
                        }
                        
                        egui::ProgressBar::new(clap_timer.0.percent()).desired_width(100.0).ui(ui);

                    }
                }

                
            });
    }

    egui::Window::new("Store").show(contexts.ctx_mut(), |ui| {
        ui.label(format!("Clicks: {}", score.stored_clicks));
        ui.label(format!("Fingers: {}", score.total_fingers));
        ui.label(format!("Multiplier: {}", score.multiplier()));
        ui.label(format!("Next Multiplier: {}", score.next_multiplier().unwrap_or(0)));
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
        if let Some(cashout) = score.cashout_cost() {
            if score.stored_clicks >= cashout {
                if ui.button(format!("Cashout ({})", cashout)).clicked() {
                    score.stored_clicks = 0;
                    score.buildings += 1;
                    score.total_fingers = 1;
                    score.total_hands = 0;
                    // delete all the hands
                    for (_, _, _, hand) in &hands {
                        commands.entity(hand).despawn_recursive();
                    }
                    // return to initial state
                    commands.spawn(Hand::default()).with_children(|parent| {
                        parent.spawn(Clicker::default());
                    });
                }
            } else {
                ui.label(format!("Cashout ({})", cashout));
            }
            
        }
    });

}

fn update_timers_system(mut all_clickers: Query<&mut TillCanClickTimer>, time: Res<Time>) {
    for mut timer in &mut all_clickers.iter_mut() {
        timer.0.tick(time.delta());
    }
}

struct ReadableImage<'a> {
    image: &'a Image,
    pixel_stride: usize,
    row_stride: usize,
}

impl ReadableImage<'_> {
    fn new(image: &Image) -> ReadableImage {
        let pixel_stride = image.data.len() / (image.size().x * image.size().y) as usize;

        ReadableImage {
            image,
            pixel_stride,
            row_stride: (image.size().x as usize) * pixel_stride,
        }
    }

    fn with_nonzero<T : FnMut(f32, f32, &[u8])>(&self, rect: Rect, mut f: T) {
        let image_rect = Rect { min: Vec2::ZERO, max: self.image.size().as_vec2() };
        let rect = image_rect.intersect(rect);
        let minx = rect.min.x as usize;
        let maxx = rect.max.x as usize;
        let miny = rect.min.y as usize;
        let maxy = rect.max.y as usize;
        
        // output center
        let center = Vec2::new((maxx - minx) as f32 / 2.0, (maxy - miny) as f32 / 2.0);


        for y in miny..maxy {
            for x in minx..maxx {
                let offset = (y * self.row_stride) + (x * self.pixel_stride);
                let pixel = &self.image.data[offset..offset + self.pixel_stride];
                
                if pixel.iter().any(|&x| x != 0) {
                    let x = (x - minx) as f32;
                    let y = (y - miny) as f32;

                    // invert and center y
                    let y = (rect.height() - y) - center.y;

                    // center x
                    let x = x - center.x;
                    f(x, y, pixel);
                }
            }
        }
    }
}

fn update_loading(
    query: Query<Entity, (With<Loading>, With<Building>)>,
    asset_server: ResMut<AssetServer>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
) {
    let building = asset_server.load("building.png");
    let image = images.get(building.clone());
    if image.is_none() {
        return;
    }
    let image = image.unwrap();
    let ri = ReadableImage::new(image);
    let atlas = TextureAtlas::from_grid(building.clone(), Vec2::new(61.0, 97.0), 2, 1, None, None);
    for entity in &query {
        commands.entity(entity).remove::<Loading>();
        commands.entity(entity).with_children(|parent| {
            ri.with_nonzero(atlas.textures[1], |x, y, pixel| {
                // println!("{} {}", x, y);
                // parent.spawn(SpriteBundle {
                //     texture: asset_server.load("target.png").into(),
                //     transform: Transform::from_xyz(x, y, 1.0).with_scale(Vec3::splat(0.25)),
                //     ..SpriteBundle::default()
                // });
                if pixel[0] == 255 {
                    // facing camera
                    parent
                        .spawn(ParticleSystemBundle {
                            particle_system: ParticleSystem {
                                max_particles: 10_000,
                                texture: asset_server.load("spark.png").into(),
                                spawn_rate_per_second: 1000.0.into(),
                                initial_speed: JitteredValue::jittered(20.0, -500.0..500.0),
                                velocity_modifiers: vec![Drag(0.001.into()), Vector(VectorOverTime::Constant(Vec3::new(0.0, -10.0, 0.0)))],
                                lifetime: JitteredValue::jittered(0.1, 0.1..0.5),
                                color: ColorOverTime::Gradient(Curve::new(vec![
                                    CurvePoint::new(Color::RED, 0.0),
                                    CurvePoint::new(Color::YELLOW, 0.75),
                                    CurvePoint::new(Color::rgba(1.0, 1.0, 1.0, 0.0), 1.0),
                                ])),
                                looping: true,
                                system_duration_seconds: 10.0,
                                max_distance: Some(600.0),
                                initial_scale: 0.01.into(),
                                scale: 50.0.into(),
                                ..ParticleSystem::default()
                            },
                            transform: Transform::from_xyz(x, y, 1.0),
                            ..ParticleSystemBundle::default()
                    }).insert(BurstTimer::default());
                } else if pixel[1] == 255 {
                    // facing left
                    parent
                        .spawn(ParticleSystemBundle {
                            particle_system: ParticleSystem {
                                max_particles: 10_000,
                                emitter_shape: EmitterShape::CircleSegment(CircleSegment {
                                    opening_angle: 0.5 * std::f32::consts::PI,
                                    radius: 0.0.into(),
                                    direction_angle: std::f32::consts::PI,
                                }),
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
                            transform: Transform::from_xyz(x, y, 1.0),
                            ..ParticleSystemBundle::default()
                    }).insert(BurstTimer::default());
                } else {
                    println!("{:?}", pixel);
                }
                
            });
        });
    }
}

#[derive(Component)]
struct Loading;

#[derive(Component)]
struct Building;

fn sync_buildings(
    query: Query<Entity, With<Building>>,
    score: Res<Score>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlases: ResMut<Assets<TextureAtlas>>,
) {
    // add buildings
    let building = asset_server.load("building.png");
    let atlas_handle = TextureAtlas::from_grid(building.clone(), Vec2::new(61.0, 97.0), 2, 1, None, None);
    let atlas = atlases.add(atlas_handle);

    let existing = query.iter().count();
    let missing = score.buildings as usize - existing;

    for x_idx in existing..existing + missing {
        println!("creating building {}", x_idx);
        commands.spawn((Loading, Building, SpriteSheetBundle {
            texture_atlas: atlas.clone(),
            transform: Transform::from_xyz(-200.0 * x_idx as f32, -50.0 as f32, 0.5).with_scale(Vec3::splat(4.0)),
            ..SpriteSheetBundle::default()
        }));
    }  
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
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
        .add_systems(Update, (
            ui_system,
            update_timers_system,
            collect_score_system,
            burst_deactivator_system,
            sync_buildings,
            update_loading
        ))
        .add_systems(Startup, setup)
        .run();
}
