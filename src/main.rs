use bevy::input::Input;
use bevy::prelude::*;
use bevy::time::Stopwatch;
use bevy_ecs_tilemap::helpers::square_grid::neighbors::Neighbors;
use bevy_ecs_tilemap::prelude::*;

const MAP_SIZE: (u32, u32) = (32, 32);
const CELL_SIZE: f32 = 16.0;
const TEAM_COLORS: [Color; 4] = [
    Color::WHITE,        // empty, shouldn't be visible
    Color::YELLOW_GREEN, // neither
    Color::BLUE,         // team 1
    Color::ORANGE,       // team 2
];

#[derive(Component, Clone, Copy, Debug)]
struct Cell(usize, usize); // team, new team

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
                .insert(Cell(0, 0))
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
    tilemap_query: Query<(&TileStorage, &TilemapSize)>,
    mut tile_query: Query<(&mut TileVisible, &mut TileColor, &mut Cell)>,
    changed_query: Query<&TilePos, &Changed<Cell>>,
) {
    if ticker.0.tick(time.delta()).elapsed_secs_f64() < ticker.1 {
        return;
    }

    ticker.0.reset();

    let (tile_storage, map_size) = tilemap_query.single();

    // first loop to move cell.1 to cell.0, to actually update them
    for cell in changed_query.iter() {
        let (mut visible, mut color, mut cell) = tile_query
            .get_mut(cell)
            .expect(&format!("Tile ({x},{y}) is not a Cell component"));

        *visible = TileVisible(cell.1 != 0);
        *color = TileColor(TEAM_COLORS[cell.1]);

        cell.0 = cell.1;
        cell.1 = 0;
    }

    // second loop to update for next time
    for cell in changed_query.iter() {
        let tile_pos = &TilePos { x, y };
        let neighbors = Neighbors::get_square_neighboring_positions(tile_pos, map_size, true)
            .entities(tile_storage);

        let (team, neighbors) = {
            let neighbors = neighbors
                .iter()
                .filter(|&c| {
                    if let Ok((_, _, cell)) = tile_query.get(*c) {
                        cell.0 != 0
                    } else {
                        false
                    }
                })
                .map(|n| {
                    let (_, _, cell) = tile_query
                        .get(*n)
                        .expect(&format!("Tile ({x},{y}) is not a Cell component"));

                    cell
                });

            let mut team = 0;
            let mut count = 0;

            for neighbor in neighbors {
                count += 1;

                if team == 0 {
                    // set team to the first team of any found neighbor
                    team = neighbor.0;
                } else if team != neighbor.0 {
                    // if a neighbor is found with a different team than the first one, change team to neither and leave the loop
                    // keep going to get the full count
                    team = 1;
                }
            }

            (team, count)
        };

        let cell = tile_storage.get(tile_pos).unwrap();
        let (_, _, mut cell) = tile_query
            .get_mut(cell)
            .expect(&format!("Tile ({x},{y}) is not a Cell component"));

        if cell.0 != 0 && neighbors == 2 || neighbors == 3 {
            cell.1 = team;
        } else {
            cell.1 = 0;
        }
    }
}

fn mouse_input(
    mouse: Res<Input<MouseButton>>,
    keys: Res<Input<KeyCode>>,
    windows: Res<Windows>,
    tilemap_query: Query<(&TileStorage, &TilemapSize)>,
    mut tile_query: Query<(&mut TileVisible, &mut TileColor, &mut Cell)>,
) {
    if mouse.just_pressed(MouseButton::Left) {
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
        let (mut visible, mut color, mut cell) = tile_query
            .get_mut(cell)
            .expect(&format!("Tile ({x},{y}) is not a Cell component"));

        let new_val = if keys.pressed(KeyCode::LControl) {
            if cell.0 == 2 {
                0
            } else {
                2
            }
        } else {
            if cell.0 == 3 {
                0
            } else {
                3
            }
        };

        cell.0 = new_val;
        cell.1 = new_val;
        *color = TileColor(TEAM_COLORS[cell.1]);
        *visible = TileVisible(new_val != 0);
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
