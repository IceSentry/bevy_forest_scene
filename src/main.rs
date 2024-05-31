use bevy::{
    core_pipeline::{
        dof::DepthOfFieldSettings,
        experimental::taa::{TemporalAntiAliasBundle, TemporalAntiAliasPlugin},
        prepass::{DeferredPrepass, DepthPrepass},
        tonemapping::Tonemapping,
        Skybox,
    },
    math::{vec2, vec3, Affine2},
    pbr::{
        wireframe::{WireframeConfig, WireframePlugin},
        DefaultOpaqueRendererMethod, ExtendedMaterial, ScreenSpaceAmbientOcclusionSettings,
        ScreenSpaceReflectionsSettings, VolumetricFogSettings, VolumetricLight,
    },
    prelude::*,
    render::{
        mesh::VertexAttributeValues,
        texture::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    },
    scene::SceneInstance,
};
use camera_controller::CameraController;
use noise::{Fbm, MultiFractal, NoiseFn, Simplex};
use plane::Plane;
use rand::{rngs::StdRng, Rng, SeedableRng};

mod camera_controller;
mod plane;
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
        .add_systems(Startup, (spawn_camera, setup_terrain, water::spawn_water))
        .add_systems(
            Update,
            (
                camera_controller::camera_controller,
                customize_scene_materials,
                spawn_terrain,
                toggle_wireframe,
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

#[derive(Resource)]
struct Terrain {
    material: Handle<StandardMaterial>,
    half_size: u32,
    fbm: Fbm<Simplex>,
}

impl Terrain {
    fn get_height(&self, pos: Vec2) -> f32 {
        let scale = 0.05;
        let pos = pos * scale;
        let pos = pos.as_dvec2();
        (self.fbm.get([pos.x, 0.0, pos.y]) as f32) * 100.0
    }

    fn generate_mesh(&self) -> Mesh {
        let mut plane: Mesh = Plane {
            size: self.half_size as f32 * 2.0,
            subdivisions: self.half_size * 2,
        }
        .into();

        match plane.attribute_mut(Mesh::ATTRIBUTE_POSITION).unwrap() {
            VertexAttributeValues::Float32x3(vertices) => {
                for pos in vertices {
                    pos[1] = self.get_height(vec2(pos[0], pos[2])) as f32;
                }
            }
            _ => unreachable!(),
        }

        plane
    }
}

fn setup_terrain(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut rng = StdRng::seed_from_u64(42);

    let terrain = Terrain {
        material: asset_server.load("forest_ground/forest_ground_04_4k.gltf#Material0"),
        half_size: 200,
        fbm: Fbm::<Simplex>::new(42).set_frequency(0.05).set_octaves(4),
    };

    let tree = asset_server.load("pine_tree_game-ready.glb#Scene0");
    for x in -(terrain.half_size as i32)..terrain.half_size as i32 {
        for z in -(terrain.half_size as i32)..terrain.half_size as i32 {
            let terrain_height = terrain.get_height(vec2(x as f32, z as f32));
            if terrain_height < 0.0 || rng.gen_range(0.0..1.0) < 0.95 {
                continue;
            }

            // add a random offset to make it less grid like
            let random_offset = vec3(rng.gen_range(-0.5..0.5), 0.0, rng.gen_range(-0.25..0.25));
            let translation = vec3(x as f32, terrain_height, z as f32) + random_offset;

            commands.spawn((
                SceneBundle {
                    scene: tree.clone(),
                    transform: Transform::from_translation(translation)
                        .with_scale(Vec3::splat(rng.gen_range(0.12..0.15)))
                        .with_rotation(Quat::from_axis_angle(
                            Vec3::Y,
                            rng.gen_range(0.0..std::f32::consts::TAU),
                        )),
                    ..default()
                },
                CustomizeMaterial,
            ));
        }
    }

    commands.insert_resource(terrain);
}

fn spawn_terrain(
    mut commands: Commands,
    terrain: Option<Res<Terrain>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut spawned: Local<bool>,
) {
    if *spawned {
        return;
    }
    let Some(terrain) = terrain else {
        return;
    };
    let Some(material) = std_materials.get_mut(&terrain.material) else {
        return;
    };
    let Some(image) = material
        .base_color_texture
        .as_ref()
        .and_then(|t| images.get_mut(t))
    else {
        return;
    };

    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        label: Some("terrain sampler".into()),
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    material.uv_transform = Affine2::from_scale(vec2(20.0, 20.0));

    let terrain_mesh = terrain.generate_mesh();

    commands.spawn(PbrBundle {
        mesh: meshes.add(terrain_mesh),
        material: terrain.material.clone(),
        ..default()
    });
    *spawned = true;
}

#[derive(Component)]
pub struct CustomizeMaterial;

pub fn customize_scene_materials(
    mut commands: Commands,
    unloaded_instances: Query<(Entity, &SceneInstance), With<CustomizeMaterial>>,
    handles: Query<(Entity, &Handle<StandardMaterial>)>,
    mut pbr_materials: ResMut<Assets<StandardMaterial>>,
    scene_manager: Res<SceneSpawner>,
) {
    for (entity, instance) in unloaded_instances.iter() {
        if scene_manager.instance_is_ready(**instance) {
            commands.entity(entity).remove::<CustomizeMaterial>();
        }
        // Iterate over all entities in scene (once it's loaded)
        let handles = handles.iter_many(scene_manager.iter_instance_entities(**instance));
        for (_entity, material_handle) in handles {
            let Some(material) = pbr_materials.get_mut(material_handle) else {
                continue;
            };

            material.alpha_mode = AlphaMode::Mask(0.5);
            material.perceptual_roughness = 1.0;
            material.metallic = 0.0;
            material.reflectance = 0.0;
        }
    }
}

fn toggle_wireframe(
    mut wireframe_config: ResMut<WireframeConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) {
        wireframe_config.global = !wireframe_config.global;
    }
}
