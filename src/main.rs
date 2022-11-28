use bevy::input::Input;
use bevy::prelude::*;
use bevy::time::Stopwatch;
use bevy_ecs_tilemap::helpers::square_grid::neighbors::Neighbors;
use bevy_ecs_tilemap::prelude::*;

const MAP_SIZE: (u32, u32) = (32, 32);
const CELL_SIZE: f32 = 16.0;

#[derive(Component, Clone, Copy, Debug)]
struct Cell(bool, bool); // living, boutta live

#[derive(Resource)]
struct TickDuration(Stopwatch, f64);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                width: 512.0,
                height: 512.0,
                title: "Conway".to_owned(),
                ..Default::default()
            },
            ..default()
        }))
        .add_plugin(GamePlugin)
        .run();
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(TilemapPlugin)
            .insert_resource(TickDuration(Stopwatch::default(), 0.1))
            .add_startup_system(startup)
            .add_system(update_map)
            .add_system(mouse_input)
            .add_system(keyboard_input);
    }
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let texture_handle: Handle<Image> = asset_server.load("tiles.png");

    let map_size = TilemapSize {
        x: MAP_SIZE.0,
        y: MAP_SIZE.1,
    };
    let mut tile_storage = TileStorage::empty(map_size);

    let map_type = TilemapType::Square;

    let tilemap_entity = commands.spawn_empty().id();

    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };
            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    color: TileColor(Color::BLACK),
                    visible: TileVisible(false),
                    ..Default::default()
                })
                .insert(Cell(false, false))
                .id();

            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize {
        x: CELL_SIZE,
        y: CELL_SIZE,
    };
    let grid_size = tile_size.into();

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle),
        map_type,
        tile_size,
        transform: get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0),
        ..Default::default()
    });
}

fn update_map(
    time: Res<Time>,
    mut ticker: ResMut<TickDuration>,
    mut tilemap_query: Query<(&TileStorage, &TilemapSize)>,
    mut tile_query: Query<(&mut TileVisible, &mut Cell)>,
) {
    if ticker.0.tick(time.delta()).elapsed_secs_f64() < ticker.1 {
        return;
    }

    ticker.0.reset();

    for (tile_storage, map_size) in tilemap_query.iter_mut() {
        // first loop to move cell.1 to cell.0, to actually update them
        for x in 0..map_size.x {
            for y in 0..map_size.y {
                let cell = tile_storage.get(&TilePos { x, y }).unwrap();
                let (mut visible, mut cell) = tile_query
                    .get_mut(cell)
                    .expect(&format!("Tile ({x},{y}) was not a Cell component"));

                *visible = TileVisible(cell.1);

                cell.0 = cell.1;
                cell.1 = false;
            }
        }

        // second loop to update for next time
        for x in 0..map_size.x {
            for y in 0..map_size.y {
                let tile_pos = &TilePos { x, y };
                let neighbors =
                    Neighbors::get_square_neighboring_positions(tile_pos, map_size, true)
                        .entities(tile_storage);

                let neighbors = neighbors
                    .iter()
                    .filter(|&c| {
                        if let Ok((_, cell)) = tile_query.get_mut(*c) {
                            cell.0
                        } else {
                            false
                        }
                    })
                    .count();

                let cell = tile_storage.get(tile_pos).unwrap();
                let (_, mut cell) = tile_query
                    .get_mut(cell)
                    .expect(&format!("Tile ({x},{y}) is not a Cell component"));

                if neighbors < 2 {
                    cell.1 = false;
                } else if cell.0 && neighbors == 2 {
                    cell.1 = true;
                } else if neighbors == 3 {
                    cell.1 = true;
                } else {
                    cell.1 = false;
                }
            }
        }
    }
}

fn mouse_input(
    buttons: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    tilemap_query: Query<(&TileStorage, &TilemapSize)>,
    mut tile_query: Query<(&mut TileVisible, &mut Cell)>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        let window = windows.get_primary().unwrap();
        let Some(position) = window.cursor_position() else { return };

        let (x, y) = (
            (position.x / CELL_SIZE).round() as u32,
            (position.y / CELL_SIZE).round() as u32,
        );

        let (tile_storage, map_size) = tilemap_query.single();

        if x >= map_size.x || y >= map_size.y {
            return;
        }

        let cell = tile_storage.get(&TilePos { x, y }).unwrap();
        let (mut visible, mut cell) = tile_query
            .get_mut(cell)
            .expect(&format!("Tile ({x},{y}) is not a Cell component"));

        let new_val = !cell.0;

        cell.0 = new_val;
        cell.1 = new_val;
        *visible = TileVisible(new_val);
    }
}

fn keyboard_input(keys: Res<Input<KeyCode>>, mut ticker: ResMut<TickDuration>) {
    if keys.just_pressed(KeyCode::Space) {
        if ticker.0.paused() {
            ticker.0.unpause();
        } else {
            ticker.0.pause();
        }
    }
}
