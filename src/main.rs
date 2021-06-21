use nannou::prelude::*;
use clap::clap_app;
#[macro_use]
extern crate pest_derive;

mod parse;

const DEBUG_MAX: u8 = 3;

struct AppSettings {
    filename: String,
    scale: f32,
    grid_size: f32,
    debug_lvl: u8,
    treshold: f32,
    hotreloading: bool,
    commands: Vec<(usize, parse::CommentlessGCodeExpr)>,
    shift_pressed: bool,
    control_pressed: bool,
    mouse_pos: Option<Point2>,
    adding_commands: Vec<parse::CommentlessGCodeExpr>,
    deleted_command: Option<parse::CommentlessGCodeExpr>,
    current_pos: Vec<Point2>,
    saved: bool,
    current_command: u8, // only G1, G2, G3
    temp_point: Option<Point2>,
    pen_mode: bool,
}

impl AppSettings {
    /// loads a gcode file to a vector of CommentlessGCodeExpr
    fn load_file(&mut self) {
        let file = std::fs::read_to_string(&self.filename).expect(&format!("Error opening `{}`.", &self.filename));
        self.commands = parse::parse_gcode_file(&file).expect("problem parsing").iter()
            .filter_map(|(l, c)| match c {
                parse::GCodeExpr::Comment(_) => None,
                parse::GCodeExpr::Pen(_) => { self.pen_mode = !self.pen_mode; Some((*l, c.to_commentless())) },
                _ => Some((*l, c.to_commentless()))
            }).collect();
        for (_, c) in self.commands.iter().rev() {
            match c {
                parse::CommentlessGCodeExpr::Move { X: x, Y: y }
                | parse::CommentlessGCodeExpr::Arc{ CLKW: _, X: x, Y: y, I: _, J: _ } => { self.current_pos = vec![pt2(*x, *y)]; break },
                _ => {}
            }
        }
    }
}

fn main() {
    nannou::app(start_app).update(update).run();
}

/// starts the app:
/// 1. match commandline args
/// 2. if   subcommand execute it
///    else build app and continue
fn start_app(app: &App) -> AppSettings {

    let matches = clap_app!(myapp =>
        (version: "0.3.1")
        (author: "Ludwig Austermann <github.com/ludwig-austermann/gcodeplot>")
        (about: "Draw simple gcode.")
        (@arg INPUT: +required "Sets the input g-code file to use")
        (@arg debug: +takes_value -d "This enables debugging and can take values up to 3. While running, this can be changed with the key `D`.")
        (@arg scale: +takes_value -s --scale "Enlarges the grid by scale. This can be changed while running with the `+`(`=`) and `-` keys.")
        (@arg treshold: +takes_value -t --treshold "Sets the treshold of number errors in your file. Default is 1e-5.")
        (@arg windowwidth: +takes_value -w --wwidth "Set the window width. Default is 800.")
        (@arg windowheight: +takes_value -h --wheight "Set the window height. Default is 600.")
        (@arg gridsize: +takes_value -g --gridsize "Sets the size of the small grid. (Defaults to 10). The larger grid is always 5times as raw. While running, this can be changed with the key `G`.")
        (@arg hotreloading: --hot "Enables hot reloading of the g-code file. Default is off. You can alternatively update the view with the key `R`.")
        (@subcommand transform =>
            (about: "Transform all coordinates in the INPUT file.")
            (version: "0.1")
            (@arg X: +takes_value -X "Move along the X axis.")
            (@arg Y: +takes_value -Y "Move along the Y axis.")
            (@arg nX: +takes_value -x "Move along the -X axis.")
            (@arg nY: +takes_value -y "Move along the -Y axis.")
            (@arg scale: +takes_value -S "Scale everything. (Note: scaling happens before translation.)")
        )
    ).get_matches();

    if let Some(submatches) = matches.subcommand_matches("transform") {
        let filename = matches.value_of("INPUT").unwrap();
        let file = std::fs::read_to_string(filename).expect(&format!("Error opening `{}`.", filename));
        let commands = parse::parse_gcode_file(&file).expect("problem parsing");
        let dx = submatches.value_of_t("X").unwrap_or(- submatches.value_of_t("nX").unwrap_or(-0.0));
        let dy = submatches.value_of_t("Y").unwrap_or(- submatches.value_of_t("nY").unwrap_or(-0.0));
        let ds = submatches.value_of_t("scale").unwrap_or(1.0);
        let mut newcmds = Vec::with_capacity(commands.len());
        for (l, cmd) in commands {
            use parse::GCodeExpr::*;
            newcmds.push((l,
                match cmd {
                    Move { X: x, Y: y } => Move { X: x * ds + dx, Y: y * ds + dy },
                    Arc { CLKW: clkw, X: x, Y: y, I: i, J: j } => Arc {
                        CLKW: clkw, X: x * ds + dx, Y: y * ds + dy, I: i * ds, J: j * ds
                    },
                    other => other,
                }))
        }
        parse::save(&format!("{}_transformed.gcode", filename.strip_suffix(".gcode").expect("Expected gcode file")), newcmds);
    } else {
        app.new_window()
            .title("GCodePlot")
            .size(
                matches.value_of_t("windowwidth").unwrap_or(800),
                matches.value_of_t("windowheight").unwrap_or(600)
            )
            .key_pressed(handle_keypress)
            .key_released(handle_keyrelease)
            .mouse_moved(handle_mouse_move)
            .mouse_pressed(handle_mouse_press)
            .view(view)
            .build()
            .unwrap();

        //if matches.is_present("hotreloading") {
        //    app.set_loop_mode(nannou::LoopMode::Rate { update_interval: std::time::Duration::from_secs(2) });
        //} else {
        //    app.set_loop_mode(nannou::LoopMode::loop_once());
        //}
        app.set_loop_mode(nannou::LoopMode::rate_fps(1.0));
    }

    let mut settings = AppSettings {
        filename: matches.value_of("INPUT").unwrap().to_string(),
        scale: matches.value_of_t("scale").unwrap_or(1.0),
        grid_size: matches.value_of_t("gridsize").unwrap_or(10.0),
        debug_lvl: matches.value_of_t("debug").unwrap_or(0) as u8,
        treshold: matches.value_of_t("threshold").unwrap_or(1e-5),
        hotreloading: matches.is_present("hotreloading"),
        commands: Vec::new(),
        shift_pressed: false,
        control_pressed: false,
        mouse_pos: None,
        adding_commands: Vec::new(),
        deleted_command: None,
        current_pos: vec![Vector2::zero()],
        saved: true,
        current_command: 0,
        temp_point: None,
        pen_mode: false, // at first is the pen up
    };
    settings.load_file();
    settings
}

/// describes how the window is displayed
fn view(app: &App, settings: &AppSettings, frame: Frame) {
    // get canvas to draw on
    let draw = app.draw();
    let window = app.main_window();
    let win = window.rect();
    let draw_area = win.pad(20.0).pad_left(10.0).pad_top(10.0);

    // set background to blue
    draw.background().color(WHITE);

    draw_grid(&draw, &draw_area, settings.grid_size, settings.scale, 0.3, false);
    draw_grid(&draw, &draw_area, 5.0 * settings.grid_size, settings.scale, 1.0, true);

    draw_gcode(&draw, &draw_area, &settings);

    draw_overlay(&draw, &win, &settings);

    // put everything on the frame
    draw.to_frame(app, &frame).unwrap();
}

fn update(_app: &App, settings: &mut AppSettings, _update: Update) {
    if settings.hotreloading {
        settings.load_file();
    }
}

/// handle keypress events
fn handle_keypress(_app: &App, settings: &mut AppSettings, key: Key) {
    let step = if settings.control_pressed {1.0} else {0.2};
    match key {
        Key::R | Key::F5 => { settings.load_file() },
        Key::D => if settings.shift_pressed && settings.debug_lvl > 0 {
            settings.debug_lvl -= 1;
        } else if settings.debug_lvl < DEBUG_MAX {
            settings.debug_lvl += 1;
        },
        Key::G => if settings.shift_pressed && settings.grid_size > step {
            settings.grid_size = (100.0 * (settings.grid_size.round() - step)) / 100.0;
        } else {
            settings.grid_size = (100.0 * (settings.grid_size.round() + step)) / 100.0;
        }
        Key::Equals => { settings.scale += step },
        Key::Minus => { if settings.scale > step { settings.scale -= step; } },
        Key::LShift | Key::RShift => { settings.shift_pressed = true },
        Key::LControl | Key::RControl => { settings.control_pressed = true },
        Key::Key1 => { settings.current_command = 1 },
        Key::Key2 => { settings.current_command = 2 },
        Key::Key3 => { settings.current_command = 3 },
        Key::H => {
            settings.adding_commands.push(parse::CommentlessGCodeExpr::Home);
            settings.current_pos.push(Vector2::zero());
        }
        Key::Key0 => { settings.current_command = 0; settings.temp_point = None },
        Key::Z => {
            settings.deleted_command = settings.adding_commands.pop();
            match settings.deleted_command {
                Some(parse::CommentlessGCodeExpr::Pen(_)) => { settings.pen_mode = !settings.pen_mode; },
                Some(_) => { settings.current_pos.pop(); },
                None => {},
            } 
        }
        Key::Y => { if let Some(c) = settings.deleted_command {
            settings.adding_commands.push(c);
            match c {
                parse::CommentlessGCodeExpr::Home => settings.current_pos.push(Vector2::zero()),
                parse::CommentlessGCodeExpr::Move { X: x, Y: y }
                | parse::CommentlessGCodeExpr::Arc{ CLKW: _, X: x, Y: y, I: _, J: _ } => settings.current_pos.push(pt2(x, y)),
                parse::CommentlessGCodeExpr::Pen(_) => { settings.pen_mode = !settings.pen_mode; }
            }
            settings.deleted_command = None;
        }},
        Key::S => { parse::resave(&settings.filename, &settings.adding_commands); settings.saved = true; },
        Key::P => {
            settings.pen_mode = !settings.pen_mode;
            settings.adding_commands.push(parse::CommentlessGCodeExpr::Pen(settings.pen_mode));
        },
        Key::F1 => { println!("For help visit: https://github.com/ludwig-austermann/gcodeplot/blob/main/readme.md") }
        _ => {}
    }
}

/// handle key release events (-> for holding shift and ctrl keys)
fn handle_keyrelease(_app: &App, settings: &mut AppSettings, key: Key) {
    match key {
        Key::LShift | Key::RShift => { settings.shift_pressed = false },
        Key::LControl | Key::RControl => { settings.control_pressed = false },
        _ => {}
    }
}

/// show the coordinates under the cursor
fn handle_mouse_move(app: &App, settings: &mut AppSettings, pos: Point2) {
    let draw_area = app.main_window().rect().pad(20.0).pad_left(10.0).pad_top(10.0);
    if draw_area.contains(pos) {
        settings.mouse_pos = Some(pos);
    } else {
        settings.mouse_pos = None;
    }
}

fn handle_mouse_press(app: &App, settings: &mut AppSettings, button: MouseButton) {
    match button {
        MouseButton::Left => if let Some(pos) = settings.mouse_pos {
            let (_, p) = get_grid_node(pos, &app.main_window().rect(), settings);
            match settings.current_command {
                1 => {
                    settings.adding_commands.push(parse::CommentlessGCodeExpr::Move { X: p.x, Y: p.y });
                    settings.current_pos.push(p);
                    settings.saved = false;
                },
                2 => {
                    if let Some(pos) = settings.temp_point {
                        settings.adding_commands.push(parse::CommentlessGCodeExpr::Arc {
                            CLKW: true, X: pos.x, Y: pos.y,
                            I: p.x - settings.current_pos.last().unwrap().x, J: p.y - settings.current_pos.last().unwrap().y
                        });
                        settings.current_pos.push(pos);
                        settings.saved = false;
                        settings.temp_point = None;
                    } else {
                        settings.temp_point = Some(p);
                    }
                },
                3 => {
                    if let Some(pos) = settings.temp_point {
                        settings.adding_commands.push(parse::CommentlessGCodeExpr::Arc {
                            CLKW: false, X: pos.x, Y: pos.y,
                            I: p.x - settings.current_pos.last().unwrap().x, J: p.y - settings.current_pos.last().unwrap().y
                        });
                        settings.current_pos.push(pos);
                        settings.saved = false;
                        settings.temp_point = None;
                    } else {
                        settings.temp_point = Some(p);
                    }
                },
                _ => {},
            }
        },
        MouseButton::Right => if let Some(pos) = settings.mouse_pos {
            let (_, p) = get_grid_node(pos, &app.main_window().rect(), settings);
            println!("X{} Y{}", p.x, p.y);
        }
        _ => {},
    }
}

/// draw the gcode on the given window.
fn draw_gcode(draw: &Draw, win: &Rect, settings: &AppSettings) {
    let mut current = pt2(0.0, 0.0);
    let origin = vec2(win.left(), win.bottom());
    let mut is_pen_down = false;
    for (l, cmd) in &settings.commands {
        use parse::CommentlessGCodeExpr::*;
        match cmd {
            Home => {
                if is_pen_down {
                    draw.line().points(current * settings.scale + origin, origin).color(BLACK).weight(2.0);
                } else if settings.debug_lvl > 0 {
                    draw.line().points(current * settings.scale + origin, origin).rgb(0.7, 0.7, 0.7);
                }
                current = Vector2::zero();
            },
            Move {X: x, Y: y}  => {
                let p = pt2(*x, *y);
                if settings.debug_lvl > 2 {
                    if is_pen_down {
                        draw.arrow().points(current * settings.scale + origin, p * settings.scale + origin).color(BLACK).weight(2.0);
                    } else {
                        draw.arrow().points(current * settings.scale + origin, p * settings.scale + origin).rgb(0.7, 0.7, 0.7).head_width(3.0);
                    }
                } else {
                    if is_pen_down {
                        draw.line().points(current * settings.scale + origin, p * settings.scale + origin).color(BLACK).weight(2.0);
                    } else if settings.debug_lvl > 0 {
                        draw.line().points(current * settings.scale + origin, p * settings.scale + origin).rgb(0.7, 0.7, 0.7);
                    }
                }
                current = p;
            },
            Pen(down) => { is_pen_down = *down; },
            Arc {CLKW: clkw, X: x, Y: y, I: i, J: j} => {
                #[allow(non_snake_case)]
                let B = pt2(*x, *y);
                #[allow(non_snake_case)]
                let C = pt2(*i, *j);
                if settings.debug_lvl > 1 {
                    let a = current * settings.scale + origin;
                    let b = B * settings.scale + origin;
                    let c = (current + C) * settings.scale + origin;
                    draw.ellipse().xy(b).w_h(4.0, 4.0).color(BLACK);
                    draw.line().points(a, c).color(RED).weight(0.3);
                    draw.ellipse().xy(c).w_h(5.0, 5.0).color(RED);
                    draw.line().points(c, b).color(RED).weight(0.3);
                    draw.ellipse().xy(a).w_h(4.0, 4.0).color(BLACK);
                }
                let a = - C;
                let r2 = a.magnitude2();
                let steps = ((r2.sqrt() * 7.2) as usize).min(36);
                let translation = (current + C) * settings.scale + origin;
                let anglestep = if (B - current).magnitude2() < settings.treshold { // make circle
                    2.0 * PI / steps as f32
                } else {
                    let b = a + B - current;
                    if (r2 - b.magnitude2()).abs() > settings.treshold {
                        println!("Cannot draw arc in line {}, (I,J) is no center.", l + 1)
                    }
                    let mut anglediff = a.angle_between(b);
                    if *clkw {
                        if (a.rotate(anglediff) - b).magnitude2() < settings.treshold { // rotate `a` in G3 direction
                            anglediff = 2.0 * PI - anglediff;
                        }
                        -anglediff / steps as f32
                    } else {
                        if (a.rotate(-anglediff) - b).magnitude2() < settings.treshold { // rotate `a` in G2 direction
                            anglediff = 2.0 * PI - anglediff;
                        }
                        anglediff / steps as f32
                    }
                };
                
                let points = (0..=steps).map(|n| a.rotate(n as f32 * anglestep) * settings.scale + translation);
                if is_pen_down {
                    draw.polyline().weight(2.0).points(points).color(BLACK);
                } else if settings.debug_lvl > 0 {
                    draw.polyline().points(points).rgb(0.7, 0.7, 0.7);
                }
                current = B;
            },
        }
    }
    for (l, cmd) in settings.adding_commands.iter().enumerate() {
        use parse::CommentlessGCodeExpr::*;
        match cmd {
            Home => {
                if is_pen_down {
                    draw.line().points(current * settings.scale + origin, origin).color(BLACK).weight(2.0);
                } else if settings.debug_lvl > 0 {
                    draw.line().points(current * settings.scale + origin, origin).rgb(0.7, 0.7, 0.7);
                }
                current = Vector2::zero();
            },
            Move {X: x, Y: y}  => {
                let p = pt2(*x, *y);
                if settings.debug_lvl > 2 {
                    if is_pen_down {
                        draw.arrow().points(current * settings.scale + origin, p * settings.scale + origin).color(BLACK).weight(2.0);
                    } else {
                        draw.arrow().points(current * settings.scale + origin, p * settings.scale + origin).rgb(0.7, 0.7, 0.7).head_width(3.0);
                    }
                } else {
                    if is_pen_down {
                        draw.line().points(current * settings.scale + origin, p * settings.scale + origin).color(BLACK).weight(2.0);
                    } else if settings.debug_lvl > 0 {
                        draw.line().points(current * settings.scale + origin, p * settings.scale + origin).rgb(0.7, 0.7, 0.7);
                    }
                }
                current = p;
            },
            Pen(down) => { is_pen_down = *down; },
            Arc {CLKW: clkw, X: x, Y: y, I: i, J: j} => {
                #[allow(non_snake_case)]
                let B = pt2(*x, *y);
                #[allow(non_snake_case)]
                let C = pt2(*i, *j);
                if settings.debug_lvl > 1 {
                    let a = current * settings.scale + origin;
                    let b = B * settings.scale + origin;
                    let c = (current + C) * settings.scale + origin;
                    draw.ellipse().xy(b).w_h(4.0, 4.0).color(BLACK);
                    draw.line().points(a, c).color(RED).weight(0.3);
                    draw.ellipse().xy(c).w_h(5.0, 5.0).color(RED);
                    draw.line().points(c, b).color(RED).weight(0.3);
                    draw.ellipse().xy(a).w_h(4.0, 4.0).color(BLACK);
                }
                let a = - C;
                let r2 = a.magnitude2();
                let steps = ((r2.sqrt() * 3.6) as usize).max(18);
                let translation = (current + C) * settings.scale + origin;
                let anglestep = if (B - current).magnitude2() < settings.treshold { // make circle
                    2.0 * PI / steps as f32
                } else {
                    let b = a + B - current;
                    if (r2 - b.magnitude2()).abs() > settings.treshold {
                        println!("Cannot draw arc in new line {}, (I,J) is no center.", l + 1)
                    }
                    let mut anglediff = a.angle_between(b);
                    if *clkw {
                        if (a.rotate(anglediff) - b).magnitude2() < settings.treshold { // rotate `a` in G3 direction
                            anglediff = 2.0 * PI - anglediff;
                        }
                        -anglediff / steps as f32
                    } else {
                        if (a.rotate(-anglediff) - b).magnitude2() < settings.treshold { // rotate `a` in G2 direction
                            anglediff = 2.0 * PI - anglediff;
                        }
                        anglediff / steps as f32
                    }
                };
                
                let points = (0..=steps).map(|n| a.rotate(n as f32 * anglestep) * settings.scale + translation);
                if is_pen_down {
                    draw.polyline().weight(2.0).points(points).color(BLACK);
                } else if settings.debug_lvl > 0 {
                    draw.polyline().points(points).rgb(0.7, 0.7, 0.7);
                }
                current = B;
            },
        }
    }
}

/// creates a grid together with coordinate system
fn draw_grid(draw: &Draw, win: &Rect, step: f32, scale: f32, weight: f32, make_axis: bool) {
    let step_by = || (0..).map(|i| i as f32 * step);
    let x_0 = win.left();
    let y_0 = win.bottom();
    for (i, x) in step_by().map(|s| x_0 + s * scale).take_while(|&f| f < win.right()).enumerate() {
        draw.line()
            .weight(weight)
            .rgb(0.9, 0.9, 0.9)
            .points(pt2(x, win.bottom()), pt2(x, win.top()));
        if make_axis {
            draw.text(&(i as f32 * step).to_string()).x_y(x, y_0 - 5.0).color(BLACK);
        }
    }
    for (i, y) in step_by().map(|s| y_0 + s * scale).take_while(|&f| f < win.top()).enumerate() {
        draw.line()
            .weight(weight)
            .rgb(0.9, 0.9, 0.9)
            .points(pt2(win.left(), y), pt2(win.right(), y));
        if make_axis {
            draw.text(&(i as f32 * step).to_string()).x_y(x_0 - 15.0, y).w(20.0).right_justify().color(BLACK);
        }
    }
    if make_axis {
        draw.line()
            .points(pt2(x_0, y_0), pt2(win.right() + 5.0, y_0));
        draw.line()
            .points(pt2(x_0, y_0), pt2(x_0, win.top() + 5.0));
    }
}

/// draws the bar with informations as well as the mouse +
fn draw_overlay(draw: &Draw, win: &Rect, settings: &AppSettings) {
    if let Some(pos) = settings.mouse_pos {
        let (pos, p) = get_grid_node(pos, win, settings);
        draw.text(&format!("mouse: ({:.2}, {:.2})", p.x, p.y))
            .x_y(win.left() + 85.0, win.top() - 5.0).w(150.0).color(BLACK).left_justify();
        // draw crosshair
        if settings.pen_mode {
            draw.line().points(pos - pt2(3.0, 0.0), pos + pt2(3.0, 0.0));
            draw.line().points(pos - pt2(0.0, 3.0), pos + pt2(0.0, 3.0));
        } else {
            draw.line().points(pos - pt2(3.0, 0.0), pos + pt2(3.0, 0.0)).rgb(0.7, 0.7, 0.7);
            draw.line().points(pos - pt2(0.0, 3.0), pos + pt2(0.0, 3.0)).rgb(0.7, 0.7, 0.7);
        }
        match settings.current_command {
            1 => { draw.text("G1-XY").xy(pos + pt2(15.0, -5.0)).w(30.0).color(RED).left_justify(); },
            2 => if settings.temp_point.is_none() {
                draw.text("G2-XY").xy(pos + pt2(15.0, -5.0)).w(30.0).color(RED).left_justify();
            } else {
                draw.text("G2-IJ").xy(pos + pt2(15.0, -5.0)).w(30.0).color(RED).left_justify();
            },
            3 => if settings.temp_point.is_none() {
                draw.text("G3-XY").xy(pos + pt2(15.0, -5.0)).w(30.0).color(RED).left_justify();
            } else {
                draw.text("G3-IJ").xy(pos + pt2(15.0, -5.0)).w(30.0).color(RED).left_justify();
            },
            _ => {}
        }
    }
    // info about state
    draw.text(&format!("scale: {:.1}", settings.scale))
        .x_y(win.left() + 220.0, win.top() - 5.0).w(80.0).color(BLACK).left_justify();
    draw.text(&format!("debug level: {}", settings.debug_lvl))
        .x_y(win.left() + 300.0, win.top() - 5.0).w(100.0).color(BLACK).left_justify();
    draw.text(&format!("grid size: {:.1}", settings.grid_size))
        .x_y(win.left() + 400.0, win.top() - 5.0).w(100.0).color(BLACK).left_justify();
    draw.text(&format!("hot reloading: {}", settings.hotreloading))
        .x_y(win.left() + 520.0, win.top() - 5.0).w(150.0).color(BLACK).left_justify();
    draw.text(&format!("accuracy treshold: {}", settings.treshold))
        .x_y(win.left() + 670.0, win.top() - 5.0).w(200.0).color(BLACK).left_justify();
}

/// calculates the nearest corresponding point on the grid. (nannou coords, plotter cords)
fn get_grid_node(pos: Point2, win: &Rect, settings: &AppSettings) -> (Point2, Point2) {
    let draw_area = win.pad(20.0).pad_left(10.0).pad_top(10.0);
    let p = (pos - draw_area.corner_at_index(3).unwrap()) / settings.scale;
    let p = p.map(|x| {
        let n = f32::trunc( x / settings.grid_size );
        if x % settings.grid_size <= 0.5 * settings.grid_size {
            n * settings.grid_size
        } else {
            (n + 1.0) * settings.grid_size
        }
    });
    let pos = p * settings.scale + draw_area.corner_at_index(3).unwrap();
    (pos, p)
}