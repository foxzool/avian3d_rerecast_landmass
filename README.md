# Avian3D Rerecast Landmass Integration

A Bevy demo showcasing integration between:
- **Avian3D**: Physics engine with character controller
- **bevy_rerecast**: Navmesh generation 
- **bevy_landmass**: AI navigation and pathfinding

## Features

- Physics-based character movement with collision detection
- Automatic navmesh generation from 3D scenes
- AI pathfinding with slope handling
- Seamless switching between manual and AI control
- Debug visualization for navigation meshes

## Controls

### Navigation
- **Left Click**: Set navigation target (agent will pathfind to clicked location)
- **WASD / Arrow Keys**: Manual character control (overrides AI navigation)

### Debug & Generation  
- **F1**: Generate navmesh from scene geometry
- **F2**: Check agent state (console output)
- **F12**: Toggle landmass debug visualization
- **Space**: Jump (when grounded)

## Implementation Details

The character uses Avian3D's kinematic character controller with:
- Slope climbing and collision response
- Physics-based movement with proper gravity
- Navigation velocity synchronized with physics engine
- Automatic target clearing on manual input

Navigation meshes are generated using bevy_rerecast and converted to bevy_landmass format for pathfinding.
