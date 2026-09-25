# Avro IDL

This project is a parser for [Avro IDL](https://avro.apache.org/docs/1.11.1/idl-language/) written in Rust. The project can emit Avro Protocol files `.avpr`.

Currently, not the full IDL is supported see the section on limitations below.

## Installation

In order to install the project, you will need to build it from source:

1. `git clone https://github.com/kvedes/avro-idl.git`
2. `cargo build --release`
3. Copy the executable into your PATH

## Getting started

Once the binary is compiled, you simply invoke it with an input path for your `avdl` file and an output path for your `avpr` file. The arguments can be seen here:

```
Usage: avro-idl <PATH> <OUTPUT_PATH> [FORMAT]

Arguments:
  <PATH>
  <OUTPUT_PATH>
  [FORMAT]       [possible values: avpr]
```

### Example

Given some simple Avro IDL file, let's call it `simple.avdl`:

```
protocol Event {
  record Person {
    string name;
    int age;
  }
}
```

It can be parsed into an `avpr` file using `avro-idl` like so:

```
avro-idl simple.avdl simple.avpr
```

This will create a file called `simple.avpr` which will have the contents:

```json
{
  "protocol": "Event",
  "types": [
    {
      "type": "record",
      "name": "Person",
      "fields": [
        {
          "name": "name",
          "type": "string"
        },
        {
          "name": "age",
          "type": "int"
        }
      ]
    }
  ]
}
```

## Supported features

The table below contains the types that are supported and whether they can be set as nullable and if they support a default value. Nullable in this case refers to shorthand notation using a question mark e.g. `int?`.

| Type      | Nullability | Default value |
| --------- | ----------- | ------------- |
| `int`     | Yes         | Yes           |
| `long`    | Yes         | Yes           |
| `float`   | Yes         | Yes           |
| `double`  | Yes         | Yes           |
| `boolean` | Yes         | Yes           |
| `string`  | Yes         | Yes           |
| `union`   | Yes         | Yes\*         |
| `record`  | Yes         | No            |
| `array`   | No          | No            |

\*: Only primitive types are supported as defaults except for string.

String defaults support JSON style escapes: `\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t` and `\uXXXX`.

### Imports

The Avro IDL protocol specifies multiple types of imports: `avsc`, `avpr` and `avdl`. This project only supports `avdl`.

### Annotations

Namespace annotations on the `protocol` are supported.

### Docstrings

Docstrings can be set for all supported types. They must start with `/**` and end with `*/`.

### Comments

Line comments (`// ...`) and block comments (`/* ... */`) are supported anywhere whitespace is allowed and are ignored by the parser.

## Unsupported

The following table contains features in the Avro IDL protocol which are not supported by this project:

| Feature                     |
| --------------------------- |
| RPC messages                |
| Errors                      |
| Records defaults using json |
| Fixed length field          |
| All logical types           |
| Maps                        |
| Ordering annotations        |
| Alias annotations           |
| Java class annotations      |

## Deviations

If a namespace is defined, it is set on all records and enums in a protocol.

