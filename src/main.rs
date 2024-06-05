use std::io::Write;

use bevy::{
    color::palettes::css::WHITE,
    core_pipeline::{
        dof::DepthOfFieldSettings,
        experimental::taa::{TemporalAntiAliasBundle, TemporalAntiAliasPlugin},
        motion_blur::MotionBlur,
        prepass::{DeferredPrepass, DepthPrepass},
        tonemapping::Tonemapping,
        Skybox,
    },
    pbr::{
        wireframe::{WireframeConfig, WireframePlugin},
        DefaultOpaqueRendererMethod, ExtendedMaterial, ScreenSpaceAmbientOcclusionSettings,
        ScreenSpaceReflectionsSettings, VolumetricFogSettings, VolumetricLight,
    },
    prelude::*,
    render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection},
    tasks::IoTaskPool,
};
use camera_controller::CameraController;
use terrain::{TerrainConfig, TerrainMaterial, TerrainResources};
use water::FoamMaterial;

mod camera_controller;
mod plane;
mod terrain;
mod water;

fn main() {
    App::new()
        .insert_resource(Msaa::Off)
        .insert_resource(DefaultOpaqueRendererMethod::deferred())
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: (1920.0, 1080.0).into(),
                    ..default()
                }),
                ..default()
            }),
            TemporalAntiAliasPlugin,
            WireframePlugin,
            MaterialPlugin::<FoamMaterial>::default(),
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, water::Water>>::default(),
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, TerrainMaterial>>::default(),
        ))
        .insert_resource(WireframeConfig {
            global: false,
            ..default()
        })
        .insert_resource(AmbientLight {
            color: Color::srgb(1.0, 1.0, 1.0),
            brightness: 0.0,
        })
        .register_type::<TerrainConfig>()
        .register_type::<SceneConfig>()
        .add_systems(
            Startup,
            (
                spawn_camera,
                terrain::setup_terrain_resources,
                water::spawn_water,
                // save_scene_system,
                terrain::load_terrain_config,
                load_scene_config,
            ),
        )
        .add_systems(
            Update,
            (
                camera_controller::camera_controller,
                terrain::customize_tree_material,
                toggle_wireframe,
                terrain::on_terrain_config_loaded.run_if(
                    resource_exists::<TerrainResources>
                        .and_then(resource_exists_and_changed::<TerrainConfig>),
                ),
                terrain::on_terrain_resource_loaded.run_if(
                    resource_exists::<TerrainResources>.and_then(resource_exists::<TerrainConfig>),
                ),
                on_scene_config_loaded.run_if(resource_exists_and_changed::<SceneConfig>),
            ),
        )
        .run();
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
struct SceneConfig {
    env_map_intensity: f32,
    skybox_brightness: f32,
    fog_color: Color,
    fog_ambient_intensity: f32,
    fog_light_intensity: f32,
    directional_light_color: Color,
    directional_light_looking_to: Vec3,
    tonemapping: Tonemapping,
    motion_blur_shutter_angle: f32,
    motion_blur_samples: u32,
    ssr: ScreenSpaceReflectionsSettings,
    camera_walk_speed: f32,
    color_grading: ColorGradingSection,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            env_map_intensity: 2000.0,
            skybox_brightness: 2000.0,
            fog_color: WHITE.into(),
            fog_ambient_intensity: 0.1,
            fog_light_intensity: 1.5,
            directional_light_color: Srgba::new(1.0, 0.75, 0.0, 1.0).into(),
            directional_light_looking_to: Vec3::new(-10.0, -1.0, 7.0),
            tonemapping: Tonemapping::default(),
            motion_blur_shutter_angle: 0.5,
            motion_blur_samples: 1,
            ssr: ScreenSpaceReflectionsSettings::default(),
            camera_walk_speed: CameraController::default().walk_speed,
            color_grading: Default::default(),
        }
    }
}

fn spawn_camera(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            Camera3dBundle {
                transform: Transform::from_xyz(0.0, 20.0, 20.0)
                    .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
                camera: Camera {
                    hdr: true,
                    ..default()
                },
                ..default()
            },
            EnvironmentMapLight {
                diffuse_map: asset_server.load("skybox/kloppenheim_01_puresky_4k_cubemap.ktx2"),
                specular_map: asset_server.load("skybox/kloppenheim_01_puresky_4k_cubemap.ktx2"),
                intensity: 2000.0,
            },
            Skybox {
                image: asset_server.load("skybox/kloppenheim_01_puresky_4k_cubemap.ktx2"),
                brightness: 2000.0,
            },
            CameraController::default(),
            VolumetricFogSettings::default(),
            DepthPrepass,
            DeferredPrepass,
            ScreenSpaceReflectionsSettings::default(),
            ScreenSpaceAmbientOcclusionSettings::default(),
            DepthOfFieldSettings::default(),
            MotionBlur::default(),
        ))
        .insert(Tonemapping::AcesFitted)
        .insert(TemporalAntiAliasBundle::default());

    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                color: Srgba::new(1.0, 0.75, 0.0, 1.0).into(),
                illuminance: 10_000.0,
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::default().looking_to(Vec3::new(-10.0, -1.0, 7.0), Vec3::Y),
            ..default()
        },
        VolumetricLight,
    ));
}

fn toggle_wireframe(
    mut wireframe_config: ResMut<WireframeConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) {
        wireframe_config.global = !wireframe_config.global;
    }
}

// This is just there in case I need another dynamic scene
fn _save_scene_system(world: &mut World) {
    let mut scene_world = World::new();
    let type_registry = world.resource::<AppTypeRegistry>().clone();
    scene_world.insert_resource(type_registry);
    scene_world.insert_resource(SceneConfig::default());
    let scene = DynamicScene::from_world(&scene_world);
    let type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = type_registry.read();
    let serialized_scene = scene.serialize(&type_registry).unwrap();

    info!("{}", serialized_scene);

    #[cfg(not(target_arch = "wasm32"))]
    IoTaskPool::get()
        .spawn(async move {
            // Write the scene RON data to file
            std::fs::File::create("assets/scene_config.scn.ron")
                .and_then(|mut file| file.write(serialized_scene.as_bytes()))
                .expect("Error while writing scene to file");
        })
        .detach();
}

fn load_scene_config(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(DynamicSceneBundle {
        scene: asset_server.load("scene_config.scn.ron"),
        ..default()
    });
}

fn on_scene_config_loaded(
    scene_config: Res<SceneConfig>,
    mut camera: Query<(
        &mut EnvironmentMapLight,
        &mut Skybox,
        &mut VolumetricFogSettings,
        &mut Tonemapping,
        &mut MotionBlur,
        &mut ScreenSpaceReflectionsSettings,
        &mut CameraController,
        &mut ColorGrading,
    )>,
    mut directional_light: Query<(&mut DirectionalLight, &mut Transform)>,
) {
    println!("scene config changed");

    for (
        mut env_map_light,
        mut skybox,
        mut fog,
        mut tonemapping,
        mut motion_blur,
        mut ssr,
        mut camera_controller,
        mut color_grading,
    ) in &mut camera
    {
        env_map_light.intensity = scene_config.env_map_intensity;
        skybox.brightness = scene_config.skybox_brightness;
        fog.ambient_intensity = scene_config.fog_ambient_intensity;
        fog.fog_color = scene_config.fog_color;
        fog.light_intensity = scene_config.fog_light_intensity;
        *tonemapping = scene_config.tonemapping;
        motion_blur.shutter_angle = scene_config.motion_blur_shutter_angle;
        motion_blur.samples = scene_config.motion_blur_samples;
        *ssr = scene_config.ssr;
        camera_controller.walk_speed = scene_config.camera_walk_speed;
        color_grading.shadows = scene_config.color_grading;
        color_grading.midtones = scene_config.color_grading;
        color_grading.highlights = scene_config.color_grading;
    }

    for (mut directional_light, mut transform) in &mut directional_light {
        directional_light.color = scene_config.directional_light_color;
        *transform = transform.looking_to(scene_config.directional_light_looking_to, Vec3::Y);
    }
}
