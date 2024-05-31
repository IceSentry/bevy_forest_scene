use bevy::{
    math::{vec2, vec3, Affine2},
    prelude::*,
    render::{
        mesh::VertexAttributeValues,
        texture::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    },
    scene::SceneInstance,
};
use noise::{Fbm, MultiFractal, NoiseFn, Simplex};
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::plane::Plane;

#[derive(Resource)]
pub struct TerrainResources {
    material: Handle<StandardMaterial>,
    tree: Handle<Scene>,
}

pub fn setup_terrain_resources(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(TerrainResources {
        material: asset_server.load("forest_ground/forest_ground_04_4k.gltf#Material0"),
        tree: asset_server.load("pine_tree_game-ready.glb#Scene0"),
    });
}

#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
pub struct TerrainConfig {
    pub half_size: u32,
    pub seed: u32,
    pub frequency: f64,
    pub octaves: usize,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            half_size: 100,
            seed: 42,
            frequency: 1.0,
            octaves: 6,
        }
    }
}

#[derive(Component)]
pub struct DespawnOnTerrainReload;

pub fn load_terrain_config(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(DynamicSceneBundle {
        scene: asset_server.load("terrain_config.scn.ron"),
        ..default()
    });
}

pub fn on_terrain_config_loaded(
    mut commands: Commands,
    terrain_config: Res<TerrainConfig>,
    terrain_resources: Option<Res<TerrainResources>>,
    despawn_on_reload: Query<Entity, With<DespawnOnTerrainReload>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Some(terrain_resources) = terrain_resources else {
        return;
    };

    println!("terrain config changed {:?}", terrain_config);

    // despawn any previous entities
    for e in &despawn_on_reload {
        commands.entity(e).despawn_recursive();
    }

    // generate terrain with loaded configs
    let fbm = Fbm::<Simplex>::new(terrain_config.seed)
        .set_frequency(terrain_config.frequency)
        .set_octaves(terrain_config.octaves);

    let mut rng = StdRng::seed_from_u64(terrain_config.seed as u64);

    for x in -(terrain_config.half_size as i32)..terrain_config.half_size as i32 {
        for z in -(terrain_config.half_size as i32)..terrain_config.half_size as i32 {
            let terrain_height = get_terrain_height(&fbm, vec2(x as f32, z as f32));
            if terrain_height < 0.0 || rng.gen_range(0.0..1.0) < 0.95 {
                continue;
            }

            // add a random offset to make it less grid like
            let random_offset = vec3(rng.gen_range(-0.5..0.5), 0.0, rng.gen_range(-0.25..0.25));
            let translation = vec3(x as f32, terrain_height, z as f32) + random_offset;

            commands.spawn((
                SceneBundle {
                    scene: terrain_resources.tree.clone(),
                    transform: Transform::from_translation(translation)
                        .with_scale(Vec3::splat(rng.gen_range(0.12..0.15)))
                        .with_rotation(Quat::from_axis_angle(
                            Vec3::Y,
                            rng.gen_range(0.0..std::f32::consts::TAU),
                        )),
                    ..default()
                },
                CustomizeTreeMaterial,
                DespawnOnTerrainReload,
            ));
        }
    }

    let terrain_mesh = generate_terrain_mesh(&fbm, terrain_config.half_size);
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(terrain_mesh),
            material: terrain_resources.material.clone(),
            ..default()
        })
        .insert(DespawnOnTerrainReload);
}

fn get_terrain_height<T: NoiseFn<f64, 2>>(fbm: &Fbm<T>, pos: Vec2) -> f32 {
    let scale = 0.05;
    let pos = pos * scale;
    let pos = pos.as_dvec2();
    (fbm.get([pos.x, pos.y]) as f32) * 100.0
}

fn generate_terrain_mesh<T: NoiseFn<f64, 2>>(fbm: &Fbm<T>, half_size: u32) -> Mesh {
    let mut plane: Mesh = Plane {
        size: half_size as f32 * 2.0,
        subdivisions: half_size * 2,
    }
    .into();

    match plane.attribute_mut(Mesh::ATTRIBUTE_POSITION).unwrap() {
        VertexAttributeValues::Float32x3(vertices) => {
            for pos in vertices {
                pos[1] = get_terrain_height(fbm, vec2(pos[0], pos[2])) as f32;
            }
        }
        _ => unreachable!(),
    }

    plane
}

pub fn fix_ground_material(
    terrain_resources: Option<Res<TerrainResources>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut spawned: Local<bool>,
) {
    if *spawned {
        return;
    }
    let Some(terrain_resources) = terrain_resources else {
        return;
    };
    let Some(material) = std_materials.get_mut(&terrain_resources.material) else {
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

    *spawned = true;
}

#[derive(Component)]
pub struct CustomizeTreeMaterial;
pub fn customize_tree_material(
    mut commands: Commands,
    unloaded_instances: Query<(Entity, &SceneInstance), With<CustomizeTreeMaterial>>,
    handles: Query<(Entity, &Handle<StandardMaterial>)>,
    mut pbr_materials: ResMut<Assets<StandardMaterial>>,
    scene_manager: Res<SceneSpawner>,
) {
    for (entity, instance) in unloaded_instances.iter() {
        if scene_manager.instance_is_ready(**instance) {
            commands.entity(entity).remove::<CustomizeTreeMaterial>();
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
