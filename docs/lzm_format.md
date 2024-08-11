Lyzumu Chart File Format Specification
======================================

## Comments
Commented lines start with `//`.

## Sections
The chart file is split into multiple sections that begins with the section name tag, `<section_name>`.
There is no strict ordering on the sections. The sections types are as follows:
- `header`
- `notes`
- `animations`
- `chart_body`

## Lines
There are four basic line types:
- Section heading line, as explained in the `Sections` section.
    - Examples
        - `<header>`
        - `<chart_body>`
- Key-value pairs to specify chart options. Behaviour depends on where the line resides in.
    - Examples
        - `key=value`
- Body line. This provides gameplay specific information with time/beat points and position. Behaviour depends on where the line resides in.
    - General format
        - `[body_type] (beat) |position| {additional_options}`
- Bar line, 
    - Only valid in chart body to separate two measures.
    - Format
        - `--`

## Header
List of header values are as follows:

- `audio_filename`
    - string, eg. `song_1.ogg`.
- `default_tempo`
    - integer, eg. `120`.
- `default_time_signature`
    - eg. `4/4`.
- `offset`. Offset to start of audio in milliseconds.
    - eg. `1231`

## Notes
Allows custom modification of note appearance. The notes types are as follows:
- `basic_n`. Color coded notes that the players must hit.
    -  `n` ranges from 1 to 4
- `target`. Notes that the player must target(collide on) and also hit.
- `flick`. Flick movement to either left or right.
- `evade_n`. Objects that the player must evade.
    -  `n` ranges from 1 to 4
- `contact_n`. Objects that the player must make contact on.
    - `n` ranges from 1 to 2

To select which note to customize, have a heading line with the note type as follows:
    - `[note_type]`

Customization options are key-value pairs and are as follows:
- `color`
    - value is color in hex. For example, `color=FFFFFF`.

## Animations
Some objects such as evade notes may be animated, i.e. have custom movement before reaching the platform's hit bar.
- Format
    - `[animation_name] (animation_type) |duration| {animation_values}`
    - `animation_name`
        - Custom user defined string to reference an animation to in the main chart body.
    - `animation_type
        - `t` for translation
        - `r` for rotation
        - `s` for scale
    - `duration`
        - Animation duration.
        - Currently only supports milliseconds as float, eg. `0.5`.
    - `animation_values`
        - for `t` and `s`, start and end vec3 values with format:
            - `{v0.x,v0.y,v0.z;v1.x,v1.y,v1.z}`
        - for `r`, rotation value per axis as vec3 and center of rotaton as vec3:
            - `{r.x,r.y,r.z;c.x, c.y, c.z}`
- Behavour
    - All interpolation is linear.
    - The object's initial position in the playfield/platform will be the initial position of the animation
    - The initial position itself is not explicitly defined. The inverse transformation is calculated to determine the initial object position.
      Animations are given by providing the hit timing/beat.

## Chart body

- Measure are separated by bar lines, `--`. Each measure can have the following k-v options:
    - `time_signature=int/int`. If not provided, uses the previous measure's time signature. For the first measure, if not provided, follows the header's `default_time_signature`.
    - `tempo=int`. Tempo changes in a measure are not supported. Behaviour is similar to `time_signature`, if not provided for this measure. 
    - `subdivision=int`. The subdivision to specify the measure's notes at. The default is the the time signature's note value(denominator) if not provided.

### Chart body objects
- The objects have the following general format:
    - `[body_type] (beat) |position| {additional_options}`
- `beat` gives the time where the object touches the hit bar.
    - Has a generic format `(b)` where b is a float and is used in conjunction with the measure's subdivision.
    - When more than one beat is required, such as hold notes, beats are separated by `;`. For example, `(2;3.5)`. The second beat signifies the end beat position.
    - To specify an end beat position that resides in another bar, provide the the number of bars to skip over after the subdivision.
        - For example, `(2;3.5,8)` means the end beat position is 8 bars ahead at subdivision 3.5.
- `position` gives the position in the platform or playfield.
    - Has a generic format `|x,y,z|` that represents a vector with float values. Multiple vectors may be provided, separated by `;`. 
        - If only two floats are provided within a vector, then is assumed to be `|x,z|` with `y`=0;
        - If only one float is provided within a vector, then is assumed to be `|x|` with `y`=0 and `z`=0;
- `{additional_options}` may be completely omitted if no additional options are required or to specify to use default options, if it exists.

#### Platform
- Rectangular platform.
    - `body_type` values are `PR` and `PRS`. `PR` signifies a platform with start and end beat position. `PRS` signifies a static platform with no explicit end position. End beat position is implicitly defined by the start position of the next platform, if it exists.
    - `beat` uses generic format with either one or two values.
    - `position` uses generic format with two vectors and only one dimension, the x position. The x values signifies the rect points.
    - `additional_options` is not used.

- Quad platform.
    - `body_type` is `PQ`.
    - `beat` uses generic format with two values, start and end beat.
    - `position` uses generic format with four vectores and one x dimension. Describes a quad in this order:
        - |bottom_left, bottom_right, top_left, top_right|
    - `additional_options` is not used.

- Curved platform.
    - `body_type` is `PC`.
    - `beat` uses generic format with two values, start and end beat.
    - `position` uses generic format with four vectores and one x dimension. Platform vertices are described in this order:
        - |bottom_left, bottom_right, top_left, top_right|
    - `additional_options` specifies the platform edges.
        - Contains two section, separated by `;`.
            - `{left_edge;right_edge}`
        - Each edge section is either one of:
            - Empty. This signifies a straight edge.
            - Contains 2 values separated by `:` that describes the bezier control points.
                - Each value contains a float that describes x position and a "beat position" similar to the `beat` section, separated by `,`. This beat section can reside in another bar.
                - For example, `{-0.5,2.5 : -0.25,3.5 ; 0.5,2.5 : 0.8,1.0,2}`. The last bezier control point is two bars ahead.
            - `m`. This character signifies that this edge should mirror the other edge, i.e. edges are parallel.

        - Examples:
            - `{;}`, `{}`, `` all mean the same thing. Edges are stright lines or a quad.
            - `{-0.5,2.5 : -0.25,3.5 ; 0.5,2.5 : 0.8,1.0,2}`
            - `{-0.5,2.5 : -0.25,3.5 ; }`
            - `{-0.5,2.5 : -0.25,3.5 ; m}`
 
#### Basic notes
- `body_type`
    - `Bn` where is `n` one of the color coded basic note numbers.
    - For example, `[B1]`
- `beat` uses generic format with one value.
- `position` uses generic format.
- `additional_options` is not used.

#### Target notes
- `body_type`
    - `T`
- `beat` uses generic format with one value.
- `position` uses generic format.
- `additional_options` is not used.

#### Hold notes
- Same format for both basic and target hold notes
- `body_type`
    - For basic notes, `HBn` where is `n` one of the color coded basic note numbers.
        - For example, `[HB1]`.
    - For target notes, `HT`.
- `beat` uses generic format with one value. Describes starting beat position.
- `position` uses generic format.
- `additional_options` can be:
    - Empty, which signifies that hold note is rectangular.
    - Contain two bezier control points separated by `:`, similar to the curved platform format.
        - Edges are always parallel.
        - For example, `{-0.5,2.5 : -0.25,3.5}`.

#### Flick notes
- `body_type` is either `FL` or `FR`, for left and right respectively.
- `beat` uses generic format with one value.
- `position` uses generic format with two values. Values are one dimensional vectores that describe the starting and ending x positions.
- `additional_options` is not used.


#### Evade notes
- `body_type`
    - `En` where is `n` one of the color coded evad note numbers.
    - For example, `[E1]`
- `beat` uses generic format with one value.
- `position` uses generic format.
- `additional_options` can either be empty or contain a list of animation names/tags.
    - For example, `{anim_move_1, anim_move2}`.

#### Contact notes
- `body_type`
    - `Cn` where is `n` one of the color coded evad note numbers.
    - For example, `[C1]`
- `beat` uses generic format with one value.
- `position` uses generic format.
- `additional_options` is not used.

