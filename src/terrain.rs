use bevy::{
    gltf::{Gltf, GltfMesh, GltfNode},
    math::{vec2, vec3, Affine2},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::{
        mesh::VertexAttributeValues,
        render_resource::{AsBindGroup, ShaderRef, ShaderType},
        texture::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    },
    scene::SceneInstance,
};
use noise::{Fbm, MultiFractal, NoiseFn, Simplex};
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::plane::Plane;

#[derive(Resource)]
pub struct TerrainResources {
    // material: Handle<StandardMaterial>,
    // tree: Handle<Scene>,
    trees_gltf: Handle<Gltf>,
    trees: Vec<Handle<Scene>>,
}

pub fn setup_terrain_resources(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(TerrainResources {
        // material: asset_server.load("forest_ground/forest_ground_04_4k.gltf#Material0"),
        // tree: asset_server.load("japanese_spruce_trees.glb#Scene3"),
        trees_gltf: asset_server.load("fir_tree_stylized.glb"),
        trees: vec![],
    });
}

pub fn on_terrain_resource_loaded(
    mut terrain_resources: ResMut<TerrainResources>,
    gltf_assets: Res<Assets<Gltf>>,
    gltf_nodes: Res<Assets<GltfNode>>,
    gltf_meshes: Res<Assets<GltfMesh>>,
    mut scenes: ResMut<Assets<Scene>>,
    mut terrain_config: ResMut<TerrainConfig>,
    mut loaded: Local<bool>,
) {
    if *loaded {
        return;
    }
    let Some(trees_gltf) = gltf_assets.get(&terrain_resources.trees_gltf) else {
        return;
    };

    // tree 0
    let mut scene_world = World::new();
    let gltf_node = gltf_nodes.get(&trees_gltf.named_nodes["Branches"]).unwrap();
    spawn_gltf_node(&mut scene_world, gltf_node, &gltf_meshes);
    let gltf_node = gltf_nodes
        .get(&trees_gltf.named_nodes["Tree_bark"])
        .unwrap();
    spawn_gltf_node(&mut scene_world, gltf_node, &gltf_meshes);
    let scene_handle = scenes.add(Scene::new(scene_world));
    terrain_resources.trees.push(scene_handle);

    // tree 1
    let mut scene_world = World::new();
    let gltf_node = gltf_nodes
        .get(&trees_gltf.named_nodes["Branches001"])
        .unwrap();
    spawn_gltf_node(&mut scene_world, gltf_node, &gltf_meshes);
    let gltf_node = gltf_nodes
        .get(&trees_gltf.named_nodes["Tree_bark001"])
        .unwrap();
    spawn_gltf_node(&mut scene_world, gltf_node, &gltf_meshes);
    let scene_handle = scenes.add(Scene::new(scene_world));
    terrain_resources.trees.push(scene_handle);

    // tree 2
    let mut scene_world = World::new();
    let gltf_node = gltf_nodes
        .get(&trees_gltf.named_nodes["Branches002"])
        .unwrap();
    spawn_gltf_node(&mut scene_world, gltf_node, &gltf_meshes);
    let gltf_node = gltf_nodes
        .get(&trees_gltf.named_nodes["Tree_bark002"])
        .unwrap();
    spawn_gltf_node(&mut scene_world, gltf_node, &gltf_meshes);
    let scene_handle = scenes.add(Scene::new(scene_world));
    terrain_resources.trees.push(scene_handle);

    terrain_config.set_changed();

    println!("tree scene loaded");
    *loaded = true;
}

fn spawn_gltf_node(scene: &mut World, gltf_node: &GltfNode, gltf_meshes: &Assets<GltfMesh>) {
    if let Some(gltf_mesh) = &gltf_node.mesh {
        spawn_gltf_mesh(scene, gltf_mesh, gltf_meshes);
    }
    // recursion stops once there are no children
    for gltf_node in &gltf_node.children {
        spawn_gltf_node(scene, gltf_node, gltf_meshes);
    }
}

fn spawn_gltf_mesh(
    scene: &mut World,
    gltf_mesh: &Handle<GltfMesh>,
    gltf_meshes: &Assets<GltfMesh>,
) {
    let gltf_mesh = gltf_meshes.get(gltf_mesh).unwrap();
    for primitive in &gltf_mesh.primitives {
        scene.spawn(PbrBundle {
            mesh: primitive.mesh.clone(),
            material: if let Some(mat) = primitive.material.as_ref() {
                mat.clone()
            } else {
                Default::default()
            },
            ..default()
        });
    }
}

#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
pub struct TerrainConfig {
    pub half_size: u32,
    pub seed: u32,
    pub frequency: f64,
    pub octaves: usize,
    pub density: f32,
    pub max_steepness: f32,
    pub use_depth_map: bool,
    pub rotation: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            half_size: 100,
            seed: 42,
            frequency: 1.0,
            octaves: 6,
            density: 0.5,
            max_steepness: 0.5,
            use_depth_map: false,
            rotation: 0.0,
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

#[allow(clippy::too_many_arguments)]
pub fn on_terrain_config_loaded(
    mut commands: Commands,
    terrain_config: Res<TerrainConfig>,
    terrain_resources: Res<TerrainResources>,
    despawn_on_reload: Query<Entity, With<DespawnOnTerrainReload>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    asset_server: Res<AssetServer>,
) {
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

    let terrain_mesh = generate_terrain_mesh(&fbm, terrain_config.half_size);
    let terrain_mesh =
        terrain_mesh.rotated_by(Quat::from_axis_angle(Vec3::Y, terrain_config.rotation));

    if !terrain_resources.trees.is_empty() {
        let positions = terrain_mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .unwrap();
        let normals = terrain_mesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .and_then(|a| a.as_float3())
            .unwrap();
        for (pos, n) in positions.iter().zip(normals) {
            let terrain_height = pos[1];
            let steepness = Vec3::from_array(*n).cross(Vec3::Y).length();

            if terrain_height < 0.01
                || rng.gen_range(0.0..1.0) < 1.0 - terrain_config.density
                || steepness > terrain_config.max_steepness
            {
                continue;
            }

            // add a random offset to make it less grid like
            let random_offset = vec3(
                rng.gen_range(-0.25..0.25),
                rng.gen_range(-0.05..0.0),
                rng.gen_range(-0.25..0.25),
            );
            let translation = Vec3::from(*pos) + random_offset;

            commands.spawn((
                SceneBundle {
                    scene: terrain_resources.trees[rng.gen_range(0..terrain_resources.trees.len())]
                        .clone(),
                    transform: Transform::from_translation(translation)
                        .with_scale(Vec3::splat(
                            // try to scale it so trees are smaller next to water
                            rng.gen_range(0.02..0.025) * (1.0 - (terrain_height / 100.0)),
                        ))
                        .with_rotation(
                            Quat::from_axis_angle(Vec3::X, 3.0 * std::f32::consts::FRAC_PI_2)
                                .mul_quat(Quat::from_axis_angle(
                                    Vec3::Z,
                                    rng.gen_range(0.0..std::f32::consts::TAU),
                                )),
                        ),
                    ..default()
                },
                CustomizeTreeMaterial,
                DespawnOnTerrainReload,
            ));
        }
    } else {
        println!("trees not ready yet");
    }

    fn terrain_sampler() -> ImageSampler {
        ImageSampler::Descriptor(ImageSamplerDescriptor {
            label: Some("terrain sampler".into()),
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            ..ImageSamplerDescriptor::linear()
        })
    }
    commands
        .spawn(MaterialMeshBundle {
            mesh: meshes.add(terrain_mesh),
            material: terrain_materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    uv_transform: Affine2::from_scale(vec2(25.0, 25.0)),
                    base_color_texture: Some(asset_server.load_with_settings(
                        "forest_ground/textures/forest_ground_04_diff_4k.jpg",
                        |s: &mut ImageLoaderSettings| {
                            s.sampler = terrain_sampler();
                        },
                    )),
                    normal_map_texture: Some(asset_server.load_with_settings(
                        "forest_ground/textures/forest_ground_04_nor_gl_4k.jpg",
                        |s: &mut ImageLoaderSettings| {
                            s.sampler = terrain_sampler();
                        },
                    )),
                    perceptual_roughness: 1.0,
                    metallic_roughness_texture: Some(asset_server.load_with_settings(
                        "forest_ground/textures/forest_ground_04_rough_4k.jpg",
                        |s: &mut ImageLoaderSettings| {
                            s.sampler = terrain_sampler();
                        },
                    )),
                    parallax_depth_scale: 0.1,
                    parallax_mapping_method: ParallaxMappingMethod::Relief { max_steps: 4 },
                    depth_map: terrain_config.use_depth_map.then(|| {
                        asset_server.load_with_settings(
                            "forest_ground/textures/forest_ground_04_disp_4k.jpg",
                            |s: &mut ImageLoaderSettings| {
                                s.sampler = terrain_sampler();
                            },
                        )
                    }),
                    opaque_render_method: bevy::pbr::OpaqueRendererMethod::Deferred,
                    double_sided: true,
                    cull_mode: None,
                    ..Default::default()
                },
                extension: TerrainMaterial {
                    settings: TerrainMaterialSettings {
                        max_steepness: terrain_config.max_steepness,
                    },
                },
            }),
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

    plane.compute_smooth_normals();
    plane.generate_tangents().unwrap();

    plane
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

#[derive(Clone, Copy, ShaderType)]
pub struct TerrainMaterialSettings {
    max_steepness: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct TerrainMaterial {
    // #[texture(100)]
    // ground_displacement: Handle<Image>,
    #[uniform(100)]
    settings: TerrainMaterialSettings,
}

impl MaterialExtension for TerrainMaterial {
    fn deferred_fragment_shader() -> ShaderRef {
        "terrain.wgsl".into()
    }
}
