let mut parts = line.split(';').next().unwrap().split(' '); // remove comments
let first = parts.next().unwrap(); // if line is empty gets first == ""
match first {
    ""    => {},
    "G28" => {
        if is_pen_down {
            draw.line().points(current * settings.scale + origin, origin).color(BLACK).weight(2.0);
        } else if settings.debug_lvl > 0 {
            draw.line().points(current * settings.scale + origin, origin).rgb(0.7, 0.7, 0.7);
        }
        current = Vector2::zero();
    },
    "G1"  => {
        let mut x: Option<f32> = None;
        let mut y: Option<f32> = None;
        for part in parts {
            if part.starts_with('X') { x = part[1..].parse().ok(); }
            if part.starts_with('Y') { y = part[1..].parse().ok(); }
        };
        match (x, y) {
            (Some(x), Some(y)) => {
                let p = pt2(x, y);
                if is_pen_down {
                    draw.line().points(current * settings.scale + origin, p * settings.scale + origin).color(BLACK).weight(2.0);
                } else if settings.debug_lvl > 0 {
                    draw.line().points(current * settings.scale + origin, p * settings.scale + origin).rgb(0.7, 0.7, 0.7);
                }
                current = p;
            },
            _ => { println!("move/line statement in line {} is not valid.", k); }
        }
    },
    "M280" => {
        if let Some("P0") = parts.next() {
            if let Some(p) = parts.next() {
                if p.starts_with("S") {
                    if let Some(n) = p[1..].parse::<i64>().ok() {
                        is_pen_down = n >= 40;
                        continue
                    }
                }
            }
        }
        println!("setting the pen must be of form `M280 P0 Sn`, where n >= 0");
    },
    mode @ "G2" | mode @ "G3" => {
        let mut x: Option<f32> = None;
        let mut y: Option<f32> = None;
        let mut i: Option<f32> = None;
        let mut j: Option<f32> = None;
        for part in parts {
            if part.starts_with('X') { x = part[1..].parse().ok(); }
            if part.starts_with('Y') { y = part[1..].parse().ok(); }
            if part.starts_with('I') { i = part[1..].parse().ok(); }
            if part.starts_with('J') { j = part[1..].parse().ok(); }
        };
        match (x, y, i, j) {
            (Some(x), Some(y), Some(i), Some(j)) => {
                if settings.debug_lvl > 1 {
                    let a = current * settings.scale + origin;
                    let b = pt2(x, y) * settings.scale + origin;
                    let c = (current + pt2(i, j)) * settings.scale + origin;
                    draw.line().points(a, c).color(RED).weight(0.3);
                    draw.ellipse().xy(c).w_h(4.0, 4.0).color(RED);
                    draw.line().points(c, b).color(RED).weight(0.3);
                    draw.ellipse().xy(a).w_h(4.0, 4.0);
                }
                let a = - pt2(i, j);
                let r2 = a.magnitude2();
                let steps = ((r2.sqrt() * 3.6) as usize).min(18);
                let translation = (current + pt2(i,j)) * settings.scale + origin;
                let anglestep = if (pt2(x, y) - current).magnitude2() < settings.treshold { // make circle
                    2.0 * PI / steps as f32
                } else {
                    let b = a + pt2(x, y) - current;
                    if (r2 - b.magnitude2()).abs() > settings.treshold {
                        println!("Cannot draw arc in line {}, (I,J) is no center.", k)
                    }
                    let mut anglediff = a.angle_between(b);
                    if mode == "G2" {
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
                current = pt2(x, y);
            },
            _ => { println!("move/line statement in line {} is not valid.", k); }
        }
    }
    cmd   => { println!("{} in line {} not implemented.", cmd, k); }
}