use crate::character::{CharacterControllerBundle, CharacterPlugin};
use avian3d::{math::*, prelude::*};
use avian_rerecast::AvianBackendPlugin;
use bevy::{
    color::palettes::css::SILVER,
    input::common_conditions::input_just_pressed,
    prelude::*,
    remote::{http::RemoteHttpPlugin, RemotePlugin},
    window::PrimaryWindow,
};
use bevy_landmass::{
    debug::{EnableLandmassDebug, Landmass3dDebugPlugin}, prelude::ThreeD, Agent3d, Agent3dBundle, AgentDesiredVelocity3d, AgentOptions,
    AgentSettings, AgentState, AgentTarget3d, Archipelago3d, ArchipelagoRef, ArchipelagoRef3d,
    FromAgentRadius, Island, Island3dBundle, Landmass3dPlugin, NavMesh3d,
    NavMeshHandle, NavigationMesh3d,
    PointSampleDistance3d,
    Velocity3d,
};
use bevy_rerecast::{debug::DetailNavmeshGizmo, prelude::*, rerecast::PolygonNavmesh};
use std::sync::Arc;

mod character;

#[derive(Reflect, Resource, Default)]
struct NavmeshResource {
    handle: Option<Handle<Navmesh>>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugins::default())
        .add_plugins((RemotePlugin::default(), RemoteHttpPlugin::default()))
        .add_plugins((NavmeshPlugins::default(), AvianBackendPlugin::default()))
        .add_plugins(Landmass3dPlugin::default())
        .add_plugins(Landmass3dDebugPlugin::default())
        .add_plugins(CharacterPlugin)
        .add_systems(Startup, (setup_scene,).chain())
        // .add_systems(PostStartup , generate_navmesh, )
        .add_systems(
            Update,
            generate_navmesh.run_if(input_just_pressed(KeyCode::F1)),
        )
        .add_systems(
            Update,
            check_agent_state.run_if(input_just_pressed(KeyCode::F2)),
        )
        .add_systems(
            Update,
            toggle_debug.run_if(input_just_pressed(KeyCode::F12)),
        )
        .add_systems(Update, (handle_mouse_click, clear_agent_target_on_input))
        .add_systems(
            Update,
            (update_agent_velocity, move_agent_by_velocity).chain(),
        )
        .add_observer(on_navmesh_ready)
        .run();
}

#[derive(Resource)]
#[allow(dead_code)]
struct NavmeshHandle(Handle<Navmesh>);

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    nav_meshes: Res<Assets<NavMesh3d>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
) {
    let archipelago = Archipelago3d::new(AgentOptions {
        point_sample_distance: PointSampleDistance3d {
            horizontal_distance: 0.6,
            distance_above: 1.0,
            distance_below: 2.0,
            vertical_preference_ratio: 2.0,
        },
        ..AgentOptions::from_agent_radius(0.6)
    });
    let archipelago_id = commands.spawn(archipelago).id();

    let _player = commands
        .spawn((
            Mesh3d(meshes.add(Capsule3d::new(0.4, 1.0))),
            MeshMaterial3d(materials.add(Color::from(SILVER))),
            Transform::from_xyz(0.0, 2.5, 0.0),
            CharacterControllerBundle::new(Collider::capsule(0.4, 1.0), Vector::NEG_Y * 9.81 * 2.0)
                .with_movement(30.0, 0.92, 7.0, (30.0 as Scalar).to_radians()),
            Agent3dBundle {
                archipelago_ref: ArchipelagoRef3d::new(archipelago_id),
                agent: Agent3d::default(),
                settings: AgentSettings {
                    radius: 1.5,
                    desired_speed: 2.0,
                    max_speed: 3.0,
                },
            },
            // AgentTarget3d::Point(Vec3::new(4.5, 1.0, 4.5)),
        ))
        .id();

    // let mesh_1: Handle<Mesh> = meshes.add(Cuboid::default());
    // let nav_mesh_1 = nav_meshes.reserve_handle();

    // // A cube to move around
    // commands.spawn((
    //     RigidBody::Dynamic,
    //     Collider::cuboid(1.0, 1.0, 1.0),
    //     Mesh3d(mesh_1.clone()),
    //     MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
    //     Transform::from_xyz(3.0, 2.0, 3.0),
    //     Island3dBundle {
    //         island: Island,
    //         archipelago_ref: ArchipelagoRef::new(archipelago_id),
    //         nav_mesh: NavMeshHandle(nav_mesh_1.clone()),
    //     },
    // ));

    // Environment (see the `collider_constructors` example for creating colliders from scenes)
    commands.spawn((
        SceneRoot(assets.load("character_controller_demo.glb#Scene0")),
        ColliderConstructorHierarchy::new(ColliderConstructor::ConvexHullFromMesh),
        RigidBody::Static,
        Island3dBundle {
            island: Island,
            archipelago_ref: ArchipelagoRef::new(archipelago_id),
            nav_mesh: NavMeshHandle(nav_meshes.reserve_handle().clone()),
        },
    ));

    // Light
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            range: 50.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 15.0, 0.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(17.0, 9.5, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn generate_navmesh(mut commands: Commands, mut generator: NavmeshGenerator) {
    let agent_radius = 0.4;
    let agent_height = 1.8;

    let settings = NavmeshSettings::from_agent_3d(agent_radius, agent_height);
    let handle = generator.generate(settings);
    commands.spawn(DetailNavmeshGizmo::new(&handle));
    commands.insert_resource(NavmeshResource {
        handle: Some(handle),
    });

    info!("Requested navmesh generation.");
}

fn on_navmesh_ready(
    trigger: Trigger<NavmeshReady>,
    navmeshes: Res<Assets<Navmesh>>,
    mut landmass_navmeshes: ResMut<Assets<bevy_landmass::NavMesh3d>>,
    mut commands: Commands,
    island: Single<Entity, With<Island>>,
) {
    let island = island.into_inner();
    let navmesh_asset_id = trigger.event().0;

    let Some(rerecast_navmesh) = navmeshes.get(navmesh_asset_id) else {
        return;
    };
    info!("Navmesh is ready! Bridging to bevy_landmass.");

    let landmass_navmesh = rerecast_to_landsmass(rerecast_navmesh);
    let landmass_navmesh = match landmass_navmesh.validate() {
        Ok(landmass_navmesh) => landmass_navmesh,
        Err(e) => {
            error!("Landmass navmesh failed validation: {e}");
            return;
        }
    };
    let landmass_navmesh = bevy_landmass::NavMesh {
        nav_mesh: Arc::new(landmass_navmesh),
        type_index_to_node_type: Default::default(),
    };
    let landmass_navmesh_handle = landmass_navmeshes.add(landmass_navmesh);
    commands
        .entity(island)
        .insert(bevy_landmass::NavMeshHandle::<ThreeD>(
            landmass_navmesh_handle,
        ));
}

fn rerecast_to_landsmass(
    rerecast_navmesh: &bevy_rerecast::Navmesh,
) -> bevy_landmass::NavigationMesh3d {
    let orig = rerecast_navmesh.polygon.aabb.min;
    let cs = rerecast_navmesh.polygon.cell_size;
    let ch = rerecast_navmesh.polygon.cell_height;
    let to_local = Vec3::new(cs, ch, cs);
    let nvp = rerecast_navmesh.polygon.max_vertices_per_polygon as usize;

    NavigationMesh3d {
        vertices: rerecast_navmesh
            .polygon
            .vertices
            .iter()
            .map(|v| orig + v.as_vec3() * to_local)
            .collect(),
        polygons: (0..rerecast_navmesh.polygon.polygon_count())
            .map(|i| {
                rerecast_navmesh.polygon.polygons[i * nvp..][..nvp]
                    .iter()
                    .filter(|i| **i != PolygonNavmesh::NO_INDEX)
                    .map(|i| *i as usize)
                    .collect::<Vec<_>>()
            })
            .collect(),
        polygon_type_indices: rerecast_navmesh
            .polygon
            .areas
            .iter()
            .map(|a| a.0 as usize)
            .collect(),
    }
}

fn handle_mouse_click(
    mut commands: Commands,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    spatial_query: SpatialQuery,
    agent_query: Query<Entity, With<Agent3d>>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let window = window_query.single().unwrap();
    let (camera, camera_transform) = camera_query.single().unwrap();

    if let Some(cursor_position) = window.cursor_position() {
        let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
            return;
        };

        let filter = SpatialQueryFilter::default();

        if let Some(hit) = spatial_query.cast_ray(ray.origin, ray.direction, 100.0, true, &filter) {
            if let Ok(agent_entity) = agent_query.single() {
                commands.entity(agent_entity).insert(AgentTarget3d::Point(
                    ray.origin + *ray.direction * hit.distance,
                ));
                // info!("Set new target for agent: {:?}",  ray.origin + *ray.direction * hit.distance,);
            }
        }
    }
}

fn update_agent_velocity(mut agent_query: Query<(&mut Velocity3d, &AgentDesiredVelocity3d)>) {
    for (mut velocity, desired_velocity) in agent_query.iter_mut() {
        // println!("{:?}", desired_velocity.velocity());
        velocity.velocity = desired_velocity.velocity();
    }
}

fn move_agent_by_velocity(
    time: Res<Time>,
    mut agent_query: Query<(&mut Transform, &GlobalTransform, &Velocity3d)>,
) {
    for (mut transform, global_transform, velocity) in agent_query.iter_mut() {
        let local_velocity = global_transform
            .affine()
            .inverse()
            .transform_vector3(velocity.velocity);
        transform.translation += local_velocity * time.delta_secs();
    }
}

/// Clear agent navigation target when WASD movement keys are pressed
fn clear_agent_target_on_input(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    agent_query: Query<Entity, With<Agent3d>>,
) {
    let movement_keys = [
        KeyCode::KeyW,
        KeyCode::ArrowUp,
        KeyCode::KeyS,
        KeyCode::ArrowDown,
        KeyCode::KeyA,
        KeyCode::ArrowLeft,
        KeyCode::KeyD,
        KeyCode::ArrowRight,
    ];

    if movement_keys.iter().any(|&key| keyboard_input.pressed(key)) {
        for entity in agent_query.iter() {
            commands.entity(entity).remove::<AgentTarget3d>();
        }
    }
}

/// System for toggling the `EnableLandmassDebug` resource.
fn toggle_debug(mut debug: ResMut<EnableLandmassDebug>) {
    **debug = !**debug;
}

fn check_agent_state(agent_state: Query<&AgentState>) {
    for state in agent_state.iter() {
        println!("{:?}", state);
    }
}
