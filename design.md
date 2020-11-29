# Units

Base units are defined with `base unit`

```
unit kilogram
unit meter
unit second

unit pixel
unit basepair
```

Names like `m<unit>`, `k<unit>`, `μ<unit>`/`u<unit>`, ... will
automatically map to a unit with the given prefix.

(Units act like the value 1 with the given unit, so they can be used
in expressions like other variables)

# Items

## Declarations

Variables are declared by writing a name that has not been used yet,
an equals sign and its value.

In an expression, juxtaposition is multiplication.

```
distance = 12 meter
time = 5 second
speed = distance / time
```

Units can be aliased, in which case they can be used in creating nicer output
(for example `2.1 min` instead of `126 second`).

Unit aliases behave like variables in all other ways.

```
unit m = meter
unit s = second

distance = 12 m
time = 5 s
```

## Expressions / evaluations

A line can also be an expression. The expression will be evaluated.
To print an expression, the line has to begin with `>`.
This will print the symbolic expression and the evaluated value.

When `=` is used without any lone unbound variables on either side it
acts as an equality check.

Equality checks behave like assertions since an inequality implies
that some error was made.

```
speed * time = distance
```

To get the unit of an expression, square brackets are used.

```
[speed] = m / s
```

Any indented text will count as belonging to the last definition.

> TODO maybe add automatic grouping based on lines??
> 
> ```
> x = 12 ^ 12 ^
>    12 - 32 *
>    12 / 12
> ```
> 
> This would need to handle leading/trailing operators

## Functions

Functions can be defined by writing the name of the function followed by
the arguments in parenthesis, separated by `,`.

```
length(t) = speed * t
```