use std::io::Write;

use bevy::{
    core_pipeline::{
        dof::DepthOfFieldSettings,
        experimental::taa::{TemporalAntiAliasBundle, TemporalAntiAliasPlugin},
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
    tasks::IoTaskPool,
};
use camera_controller::CameraController;
use terrain::TerrainConfig;

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
        ))
        .add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, water::Water>,
        >::default())
        .insert_resource(WireframeConfig {
            global: false,
            ..default()
        })
        .insert_resource(AmbientLight {
            color: Color::srgb(1.0, 1.0, 1.0),
            brightness: 0.0,
        })
        .register_type::<TerrainConfig>()
        .add_systems(
            Startup,
            (
                spawn_camera,
                terrain::setup_terrain_resources,
                water::spawn_water,
                // save_scene_system,
                terrain::load_terrain_config,
            ),
        )
        .add_systems(
            Update,
            (
                camera_controller::camera_controller,
                terrain::customize_tree_material,
                terrain::fix_ground_material,
                toggle_wireframe,
                terrain::on_terrain_config_loaded,
            ),
        )
        .run();
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
            VolumetricFogSettings {
                ambient_intensity: 0.1,
                // fog_color: Srgba::new(1.0, 0.75, 0.0, 1.0).into(),
                // light_tint: Srgba::new(1.0, 0.75, 0.0, 1.0).into(),
                light_intensity: 1.5,
                // absorption: 0.5,
                // scattering: 0.8,
                // density: 0.3,
                ..default()
            },
            DepthPrepass,
            DeferredPrepass,
            ScreenSpaceReflectionsSettings::default(),
            ScreenSpaceAmbientOcclusionSettings::default(),
            DepthOfFieldSettings::default(),
            // ColorGrading {
            //     global: todo!(),
            //     shadows: todo!(),
            //     midtones: todo!(),
            //     highlights: todo!(),
            // },
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
    // scene_world.insert_resource(TerrainConfig {
    //     half_size: 200,
    //     seed: 42,
    //     frequency: 0.05,
    //     octaves: 6,
    // });
    let scene = DynamicScene::from_world(&scene_world);
    let type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = type_registry.read();
    let serialized_scene = scene.serialize(&type_registry).unwrap();

    info!("{}", serialized_scene);

    #[cfg(not(target_arch = "wasm32"))]
    IoTaskPool::get()
        .spawn(async move {
            // Write the scene RON data to file
            std::fs::File::create("assets/terrain_config.scn.ron")
                .and_then(|mut file| file.write(serialized_scene.as_bytes()))
                .expect("Error while writing scene to file");
        })
        .detach();
}
