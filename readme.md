# gcodeplot

*Note that this app purposely supports only a very minimal subset of g-code features, since it is primarily supposed for educational purposes.*

![Screenshot of version 0.2](img/v0.2_screenshot.png)
![Screenshot of debugging features 0.4](img/v0.4_screenshot_debugging.png)

*The pictures show a screenshot without debugging features and a zoomed screenshot of debugging features.*

## Description

`gcodeplot` is a simulated plotter of [G-code](http://en.wikipedia.org/wiki/G-code) files. It supports only
- `G28`: **return to home** position
- `G0 X{NUM} Y{NUM}`: **move**, move to `(X,Y)` (from current position) (note that you probably want to use `G1` instead)
- `G1 X{NUM} Y{NUM}`: **linear move**, move directly to `(X,Y)` (from current position)
- `G2 X{NUM} Y{NUM} I{NUM} J{NUM}`: (part-)**circular move in clockwise direction**, move to `(X,Y)` (from current position) along a circle with center in `CURRENTPOS + (I,J)`
- `G3 X{NUM} Y{NUM} I{NUM} J{NUM}`: (part-)**circular move in anticlockwise direction**, move to `(X,Y)` (from current position) along a circle with center in `CURRENTPOS + (I,J)`
- `M280 P0 S{NUM}`: **Set the pen** as follows, if `S>=40` down (which means it can draw) and else up
- `;{}`: **comment**, which can be put on seperate line or after a regular command

### Parsing Abilities

The `gcode` parser allows for arbitrary many, including zero, spacing characters (' ' & '\t') between instruction name and argument name and between argument name and number expression. It is also allowed to change the order of arguments. For example this is allowed:
```text
G0   X  10  Y  20
G1   Y  10  X  20
G1X20Y50
```
Furthermore are number expressions treated as 32 bit floats internally and parsed as such, which allows the following expressions:
```text
G1 X 00001 Y 1.000
G1 X 0.1e5 Y 1e-2
```

## Arguments And Features

The application is programmed in rust with the [nannou library](https://nannou.cc/) for displaying and the [pest library](https://pest.rs/) for parsing. It supports the following command line arguments:
- `INPUT` (required): sets the gcode file to plot
- `-d {}`, `--debug {}`: This enables debugging and can take values up to 3. While running, this can be changed with the key <kbd>D</kbd>
- `-s {}`, `--scale {}`: This scales the whole view. While running you can access it with the <kbd>+</kbd>(might be <kbd>=</kbd> on your PC) and <kbd>-</kbd> keys
- `-g {}`, `--gridsize`: This describes the gridsize used. While running you can access it with the key <kbd>G</kbd>
- the flag `--hot`, for hot reloading of the gcode file. Alternatively you can update in the app with <kbd>R</kbd>
- and other arguments as `--treshold`, `--wwidth` and `-wheight`.

It furthermore supports a subcommand `transform`, which
allows you to transform a file by translation and dilation.

## Keyboard Commands And Editing Features

In the graphical app, a few keyboard commands are enabled. To increase a value corresponding to a <kbd>key</kbd>, just press <kbd>key</kbd> and to decrease press <kbd>shift</kbd> + <kbd>key</kbd>. For bigger steps combine these combination with a further <kbd>ctrl</kbd>.

Furthermore, I introduced in Version 0.3.0 the ability to extend existing g-code files. Simply press
- <kbd>0</kbd> for `G0` mode
- <kbd>1</kbd> for `G1` mode
- <kbd>2</kbd> for `G2` mode
- <kbd>3</kbd> for `G3` mode
- <kbd>esc</kbd> to exit the modes

and choose the coordinate with a left mouse click. One also can now undo and redo these added commands with <kbd>Z</kbd> and <kbd>Y</kbd> and save these changes to a new file with <kbd>S</kbd>. <kbd>P</kbd> changes the penmode and <kbd>H</kbd> returns to to home.

Last but not least, right-clicking prints the mouse coordinates to console and <kbd>Q</kbd> quits the application.

## Room To Improve

While the app can be used for many purposes, it is in a early development phase. Tests are missing and not everything is programmed the clever way. If you want to improve it feel welcome to contribute.

TODO:
- [ ] better loop management for hot reloading
- [ ] more file maniplulation features
- [ ] move in grid support
- [X] draw mode?
- [ ] more debug options, e.g. show coordinates
- [ ] autoscale the view, using for instance <kbd>A</kbd>.
- [ ] gcode to svg output
- [ ] support `R` argument: 
- [ ] support `G90`/`G91`: rel. & abs. coordinates
- [ ] support `U{}`/`V{}`: rel. coordinates
- [ ] support `G90.1`/`G91.1`: rel. & abs. coordinates for `I` and `J` arguments
- [ ] support `G68`/`G69`: coordinate rotation
- [ ] support `O{}`, `M98`, `M99`, `P{}`, `M2`: to handle subprograms
- [ ] support `L`
- [ ] support for switching features on and off in a config toml file
- [ ] support for opinionated formating off a file, with options: minimal,