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
}

impl AppSettings {
    /// loads a gcode file to a vector of CommentlessGCodeExpr
    fn load_file(&mut self) {
        let file = std::fs::read_to_string(&self.filename).expect(&format!("Error opening `{}`.", &self.filename));
        self.commands = parse::parse_gcode_file(&file).expect("problem parsing").iter()
            .filter_map(|(l, c)| if let parse::GCodeExpr::Comment(_) = c {
                None
            } else {
                Some((*l, c.to_commentless()))
            }).collect();
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
        (version: "0.2.1")
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
    
    // info about state
    draw.text(&format!("scale: {:.1},   debug level: {},   grid size: {:.1},   hot reloading: {},   accuracy treshold: {}", settings.scale, settings.debug_lvl, settings.grid_size, settings.hotreloading, settings.treshold))
        .x_y(win.left() + 140.0, win.top() - 5.0).w(win.w() - 40.0).color(BLACK).right_justify();

    draw_grid(&draw, &draw_area, settings.grid_size, settings.scale, 0.3, false);
    draw_grid(&draw, &draw_area, 5.0 * settings.grid_size, settings.scale, 1.0, true);

    draw_gcode(&draw, &draw_area, &settings);

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
        Key::R => { settings.load_file() },
        Key::D => if settings.shift_pressed && settings.debug_lvl > 0 {
            settings.debug_lvl -= 1;
        } else if settings.debug_lvl < DEBUG_MAX {
            settings.debug_lvl += 1;
        },
        Key::G => if settings.shift_pressed && settings.grid_size > step {
            settings.grid_size = (100.0 * settings.grid_size.round()) / 100.0 - step;
        } else {
            settings.grid_size = (100.0 * settings.grid_size.round()) / 100.0 + step;
        }
        Key::Equals => { settings.scale += step },
        Key::Minus => { if settings.scale > step { settings.scale -= step; } },
        Key::LShift | Key::RShift => { settings.shift_pressed = true },
        Key::LControl | Key::RControl => { settings.control_pressed = true },
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
                        draw.arrow().points(current * settings.scale + origin, p * settings.scale + origin).rgb(0.7, 0.7, 0.7);
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
                let steps = ((r2.sqrt() * 3.6) as usize).min(18);
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