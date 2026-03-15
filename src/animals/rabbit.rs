use super::AnimalDef;

// All frames padded to 24 rows. Vertical position within the frame
// creates jump/crouch motion (gostty-style frame-baked movement).
// Max row jump between consecutive frames: 2 rows → smooth transitions.

// Frame 0: Idle — sitting, ears up, eyes center
const IDLE: &[&str] = &[
    "",
    "",
    "",
    r"      /\      /\",
    r"     /  \    /  \",
    r"     |  |    |  |",
    r"     |  |    |  |",
    r"     |  |    |  |",
    r"      \  \  /  /",
    r"       \  \/  /",
    r"     .==========.",
    r"    /              \",
    r"   |   **      **   |",
    r"   |                 |",
    r"   |       w         |",
    r"   |    .======.     |",
    r"    \              /",
    r"     '=========='",
    r"      ||      ||",
    "",
    "",
    "",
    "",
    "",
];

// Frame 1: Blink
const BLINK: &[&str] = &[
    "",
    "",
    "",
    r"      /\      /\",
    r"     /  \    /  \",
    r"     |  |    |  |",
    r"     |  |    |  |",
    r"     |  |    |  |",
    r"      \  \  /  /",
    r"       \  \/  /",
    r"     .==========.",
    r"    /              \",
    r"   |   --      --   |",
    r"   |                 |",
    r"   |       w         |",
    r"   |    .======.     |",
    r"    \              /",
    r"     '=========='",
    r"      ||      ||",
    "",
    "",
    "",
    "",
    "",
];

// Frame 2: Crouch 1 — ears slightly shorter, shifted down 2 rows
const CROUCH1: &[&str] = &[
    "",
    "",
    "",
    "",
    "",
    r"      /\      /\",
    r"     /  \    /  \",
    r"     |  |    |  |",
    r"     |  |    |  |",
    r"      \  \  /  /",
    r"       \  \/  /",
    r"     .==========.",
    r"    /              \",
    r"   |   **      **   |",
    r"   |                 |",
    r"   |       w         |",
    r"   |    .======.     |",
    r"    \              /",
    r"     '=========='",
    r"      ||      ||",
    "",
    "",
    "",
    "",
];

// Frame 3: Crouch 2 — ears very short, body compressed, shifted down 4 rows
const CROUCH2: &[&str] = &[
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    r"      /\      /\",
    r"     |  |    |  |",
    r"      \  \  /  /",
    r"       \  \/  /",
    r"     .==========.",
    r"    /              \",
    r"   |   **      **   |",
    r"   |       w         |",
    r"   |    .======.     |",
    r"    \              /",
    r"     '=========='",
    r"      ||      ||",
    "",
    "",
    "",
    "",
    "",
];

// Frame 4: Jump launch — ears closing, same height as idle
const LAUNCH: &[&str] = &[
    "",
    "",
    "",
    r"      /\    /\",
    r"     /  \  /  \",
    r"     |  |  |  |",
    r"     |  | /  /",
    r"      \  \/  /",
    r"     .==========.",
    r"    /              \",
    r"   |   **      **   |",
    r"   |                 |",
    r"   |       w         |",
    r"   |    .======.     |",
    r"    \              /",
    r"     '=========='",
    r"      ||      ||",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
];

// Frame 5: Jump transition — ears angled, shifted up 2 rows
const JUMP_TRANS: &[&str] = &[
    "",
    r"     /\    /\",
    r"    /  \  /  \",
    r"    |  | |  |",
    r"     \  \/  /",
    r"     .==========.",
    r"    /              \",
    r"   |   **      **   |",
    r"   |                 |",
    r"   |       w         |",
    r"   |    .======.     |",
    r"    \              /",
    r"     '=========='",
    r"      ||      ||",
    r"       |      |",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
];

// Frame 6: Jump peak — ears swept back, shifted up 3 rows, legs extended
const JUMP_PEAK: &[&str] = &[
    r"    --/\  /\--",
    r"       \/\/",
    r"     .==========.",
    r"    /              \",
    r"   |   **      **   |",
    r"   |                 |",
    r"   |       w         |",
    r"   |    .======.     |",
    r"    \              /",
    r"     '=========='",
    r"      ||      ||",
    r"       |      |",
    r"       |      |",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
];

// Frame 7: Look left — eyes shifted left
const LOOK_LEFT: &[&str] = &[
    "",
    "",
    "",
    r"      /\      /\",
    r"     /  \    /  \",
    r"     |  |    |  |",
    r"     |  |    |  |",
    r"     |  |    |  |",
    r"      \  \  /  /",
    r"       \  \/  /",
    r"     .==========.",
    r"    /              \",
    r"   | **        **   |",
    r"   |                 |",
    r"   |       w         |",
    r"   |    .======.     |",
    r"    \              /",
    r"     '=========='",
    r"      ||      ||",
    "",
    "",
    "",
    "",
    "",
];

// Frame 8: Look right — eyes shifted right
const LOOK_RIGHT: &[&str] = &[
    "",
    "",
    "",
    r"      /\      /\",
    r"     /  \    /  \",
    r"     |  |    |  |",
    r"     |  |    |  |",
    r"     |  |    |  |",
    r"      \  \  /  /",
    r"       \  \/  /",
    r"     .==========.",
    r"    /              \",
    r"   |     **      ** |",
    r"   |                 |",
    r"   |       w         |",
    r"   |    .======.     |",
    r"    \              /",
    r"     '=========='",
    r"      ||      ||",
    "",
    "",
    "",
    "",
    "",
];

pub const DEF: AnimalDef = AnimalDef {
    frames: &[
        IDLE,       // 0
        BLINK,      // 1
        CROUCH1,    // 2
        CROUCH2,    // 3
        LAUNCH,     // 4
        JUMP_TRANS, // 5
        JUMP_PEAK,  // 6
        LOOK_LEFT,  // 7
        LOOK_RIGHT, // 8
    ],
    // Smooth jump: max 2-row position change between consecutive frames
    // Row positions: idle=3, crouch1=5, crouch2=7, launch=3, trans=1, peak=0
    sequence: &[
        (0, 2.0),  // Idle
        (1, 0.12), // Blink
        (0, 2.0),  // Idle
        // Jump: crouch down → explode up → peak → come down → land
        (2, 0.10), // Crouch 1 (down 2)
        (3, 0.10), // Crouch 2 (down 2)
        (2, 0.08), // Crouch 1 (up 2, rising)
        (4, 0.08), // Launch (up 2, ears closing)
        (5, 0.08), // Jump trans (up 2)
        (6, 0.25), // Jump peak (up 1, hold)
        (5, 0.08), // Jump trans (down 1)
        (4, 0.08), // Launch (down 2)
        (2, 0.08), // Crouch 1 (down 2, landing)
        (3, 0.10), // Crouch 2 (down 2, impact)
        (2, 0.10), // Crouch 1 (up 2, recovery)
        (0, 2.0),  // Idle
        (1, 0.12), // Blink
        (0, 1.5),  // Idle
        // Look around
        (7, 1.0),  // Look left
        (0, 0.3),  // Idle (center)
        (8, 1.0),  // Look right
        (0, 1.3),  // Idle
    ],
    name: "Rabbit",
    color_top: (255, 190, 220),
    color_bot: (240, 140, 180),
};
