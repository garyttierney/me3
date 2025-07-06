"""Convert JSON Schema to Markdown documentation."""

__author__ = "Stéphane Brunner"
__email__ = "stephane.brunner@gmail.com"
__license__ = "Apache-2.0"


try:
    from importlib.metadata import version
except ImportError:
    from importlib_metadata import version

import argparse
import io
import json
import re
import subprocess  # nosec
from collections.abc import Sequence
from pathlib import Path
from typing import Any, Optional, Union
from urllib.parse import quote

import markdown
import yaml

__version__ = version("jsonschema2md")


class Parser:
    """
    JSON Schema to Markdown parser.

    Examples
    --------
    >>> import jsonschema2md
    >>> parser = jsonschema2md.Parser()
    >>> md_lines = parser.parse_schema(json.load(input_json))
    """

    tab_size = 2

    def __init__(
        self,
        examples_as_yaml: bool = False,
        show_examples: str = "all",
        show_deprecated: bool = False,
        collapse_children: bool = False,
        header_level: int = 0,
        ignore_patterns: Optional[Sequence[str]] = None,
    ) -> None:
        """
        Initialize JSON Schema to Markdown parser.

        Parameters
        ----------
        examples_as_yaml : bool, default False
            Parse examples in YAML-format instead of JSON.
        show_examples: str, default 'all'
            Parse examples for only objects, only properties or all. Valid options are
            `{"all", "object", "properties"}`.
        show_deprecated : bool, default False
            If `True`, includes deprecated properties in the generated markdown. This
            allows for documenting properties that are no longer recommended for use.
        collapse_children : bool, default False
            If `True`, collapses objects with children in the generated markdown. This
            allows for a cleaner view of the schema.
        header_level : int, default 0
            Base header level for the generated markdown. This is useful to include the
            generated markdown in a larger document with its own header levels.
        ignore_patterns : list of str, default None
            List of regex patterns to ignore when parsing the schema. This can be useful
            to skip certain properties or definitions that are not relevant for the
            documentation. The patterns are matched against the full path of the
            property or definition (e.g., `properties/name`, `definitions/Person`).

        """
        self.examples_as_yaml = examples_as_yaml
        self.show_deprecated = show_deprecated
        self.header_level = header_level
        self.collapse_children = collapse_children
        self.ignore_patterns = ignore_patterns if ignore_patterns else []

        valid_show_examples_options = ["all", "object", "properties"]
        show_examples = show_examples.lower()
        if show_examples in valid_show_examples_options:
            self.show_examples = show_examples
        else:
            message = (
                f"`show_examples` option should be one of "
                f"`{valid_show_examples_options}`; `{show_examples}` was passed.",
            )
            raise ValueError(message)

    def _construct_description_line(
        self, obj: dict[str, Any], add_type: bool = False
    ) -> Sequence[str]:
        """Construct description line of property, definition, or item."""
        description_line = []

        if "description" in obj:
            ending = "" if re.search(r"[.?!;]$", obj["description"]) else "."
            description_line.append(f"{obj['description']}{ending}")
        if add_type and "type" in obj:
            description_line.append(f"Must be of type *{obj['type']}*.")
        if "contentEncoding" in obj:
            description_line.append(f"Content encoding: `{obj['contentEncoding']}`.")
        if "contentMediaType" in obj:
            description_line.append(f"Content media type: `{obj['contentMediaType']}`.")
        if "minimum" in obj:
            description_line.append(f"Minimum: `{obj['minimum']}`.")
        if "exclusiveMinimum" in obj:
            description_line.append(f"Exclusive minimum: `{obj['exclusiveMinimum']}`.")
        if "maximum" in obj:
            description_line.append(f"Maximum: `{obj['maximum']}`.")
        if "exclusiveMaximum" in obj:
            description_line.append(f"Exclusive maximum: `{obj['exclusiveMaximum']}`.")
        if "minItems" in obj or "maxItems" in obj:
            length_description = "Length must be "
            if "minItems" in obj and "maxItems" not in obj:
                length_description += f"at least {obj['minItems']}."
            elif "maxItems" in obj and "minItems" not in obj:
                length_description += f"at most {obj['maxItems']}."
            elif obj["minItems"] == obj["maxItems"]:
                length_description += f"equal to {obj['minItems']}."
            else:
                length_description += (
                    f"between {obj['minItems']} and {obj['maxItems']} (inclusive)."
                )
            description_line.append(length_description)
        if "multipleOf" in obj:
            if obj["multipleOf"] == 1:
                description_line.append("Must be an integer.")
            else:
                description_line.append(f"Must be a multiple of `{obj['multipleOf']}`.")

        if "minLength" in obj or "maxLength" in obj:
            length_description = "Length must be "
            if "minLength" in obj and "maxLength" not in obj:
                length_description += f"at least {obj['minLength']}."
            elif "maxLength" in obj and "minLength" not in obj:
                length_description += f"at most {obj['maxLength']}."
            elif obj["minLength"] == obj["maxLength"]:
                length_description += f"equal to {obj['minLength']}."
            else:
                length_description += (
                    f"between {obj['minLength']} and {obj['maxLength']} (inclusive)."
                )
            description_line.append(length_description)
        if "pattern" in obj:
            link = f"https://regexr.com/?expression={quote(obj['pattern'])}"
            description_line.append(
                f"Must match pattern: `{obj['pattern']}` ([Test]({link}))."
            )
        if obj.get("uniqueItems"):
            description_line.append("Items must be unique.")
        if "minContains" in obj or "maxContains" in obj:
            contains_description = "Contains schema must be matched"
            if "minContains" in obj and "maxContains" not in obj:
                contains_description += f" at least {obj['minContains']} times."
            elif "maxContains" in obj and "minContains" not in obj:
                contains_description += f" at most {obj['maxContains']} times."
            elif obj["minContains"] == obj["maxContains"]:
                contains_description += f" exactly {obj['minContains']} times."
            else:
                contains_description += f" between {obj['minContains']} and {obj['maxContains']} times (inclusive)."

            description_line.append(contains_description)
        if "maxProperties" in obj or "minProperties" in obj:
            properties_description = "Number of properties must be "
            if "minProperties" in obj and "maxProperties" not in obj:
                properties_description += f"at least {obj['minProperties']}."
            elif "maxProperties" in obj and "minProperties" not in obj:
                properties_description += f"at most {obj['maxProperties']}."
            elif obj["minProperties"] == obj["maxProperties"]:
                properties_description += f"equal to {obj['minProperties']}."
            else:
                properties_description += f"between {obj['minProperties']} and {obj['maxProperties']} (inclusive)."
            description_line.append(properties_description)
        if "enum" in obj:
            description_line.append(f"Must be one of: `{json.dumps(obj['enum'])}`.")
        if "const" in obj:
            description_line.append(f"Must be: `{json.dumps(obj['const'])}`.")
        for extra_props in ["additional", "unevaluated"]:
            if f"{extra_props}Properties" in obj:
                if obj[f"{extra_props}Properties"]:
                    description_line.append(f"Can contain {extra_props} properties.")
                else:
                    description_line.append(f"Cannot contain {extra_props} properties.")
        if "$ref" in obj:
            ref = obj["$ref"].removeprefix("#/$defs/")
            description_line.append(f"Refer to *[{ref}](#{quote(ref)})*.")
        if "default" in obj:
            description_line.append(f"Default: `{json.dumps(obj['default'])}`.")

        # Only add start colon if items were added
        if description_line:
            description_line.insert(0, ":")

        return description_line

    def _construct_examples(
        self,
        obj: dict[str, Any],
        indent_level: int = 0,
        add_header: bool = True,
    ) -> Sequence[str]:
        def dump_json_with_line_head(
            obj: dict[str, Any], line_head: str, **kwargs: Any
        ) -> str:
            result = [
                line_head + line
                for line in io.StringIO(json.dumps(obj, **kwargs)).readlines()
            ]
            return "".join(result)

        def dump_yaml_with_line_head(
            obj: dict[str, Any], line_head: str, **kwargs: Any
        ) -> str:
            result = [
                line_head + line
                for line in io.StringIO(
                    yaml.dump(obj, sort_keys=False, **kwargs)
                ).readlines()
            ]
            return "".join(result).rstrip()

        example_lines = []
        if "examples" in obj:
            example_indentation = " " * self.tab_size * (indent_level + 1)
            if add_header:
                example_lines.append(f"\n{example_indentation}Examples:\n")
            for example in obj["examples"]:
                if self.examples_as_yaml:
                    lang = "yaml"
                    dump_fn = dump_yaml_with_line_head
                else:
                    lang = "json"
                    dump_fn = dump_json_with_line_head
                example_str = dump_fn(example, line_head=example_indentation, indent=4)
                example_lines.append(
                    f"{example_indentation}```{lang}\n{example_str}\n{example_indentation}```\n\n",
                )
        return example_lines

    def _parse_object(
        self,
        obj: Union[dict[str, Any], list[Any]],
        name: Optional[str],
        path: list[str],
        name_monospace: bool = True,
        output_lines: Optional[list[str]] = None,
        indent_level: int = 0,
        required: bool = False,
        dependent_required: Optional[list[str]] = None,
    ) -> list[str]:
        """Parse JSON object and its items, definitions, and properties recursively."""
        if not output_lines:
            output_lines = []

        indentation = " " * self.tab_size * indent_level
        indentation_items = " " * self.tab_size * (indent_level + 1)

        if isinstance(obj, list):
            output_lines.append(f"{indentation}- **{name}**:\n")

            for i, element in enumerate(obj):
                output_lines = self._parse_object(
                    element,
                    path=[*path, str(i)],
                    name=None,
                    name_monospace=False,
                    output_lines=output_lines,
                    indent_level=indent_level + 2,
                )
            return output_lines

        if not isinstance(obj, dict):
            message = f"Non-object type found in properties list: `{name}: {obj}`."
            raise TypeError(message)

        # Construct full description line
        description_line_base = self._construct_description_line(obj)
        description_line_list = [
            line.replace("\n\n", "<br>" + indentation_items)
            for line in description_line_base
        ]

        # Add full line to output
        description_line = " ".join(description_line_list)
        optional_format = f", format: {obj['format']}" if "format" in obj else ""
        if name is None:
            obj_type = f"*{obj['type']}{optional_format}*" if "type" in obj else ""
            name_formatted = ""
        else:
            required_str = ", required" if required else ""
            deprecated_str = ", deprecated" if obj.get("deprecated") else ""
            readonly_str = ", read-only" if obj.get("readOnly") else ""
            writeonly_str = ", write-only" if obj.get("writeOnly") else ""
            if dependent_required and not required:
                dependent_required_code = [f"`{k}`" for k in dependent_required]
                if len(dependent_required_code) == 1:
                    required_str += f", required <sub><sup>if {dependent_required_code[0]} is set</sup></sub>"
                else:
                    required_str += f", required <sub><sup>if {', '.join(dependent_required_code[:-1])}, or {dependent_required_code[-1]} is set</sup></sub>"
            obj_type = (
                f" *({obj['type']}{optional_format}{required_str}{deprecated_str}{readonly_str}{writeonly_str})*"
                if "type" in obj
                else ""
            )
            name_formatted = f"**`{name}`**" if name_monospace else f"**{name}**"

        has_children = any(
            prop in obj and isinstance(obj[prop], dict)
            for prop in [
                "additionalProperties",
                "unevaluatedProperties",
                "properties",
                "patternProperties",
            ]
        )

        anchor = f'<a id="{quote(path[-1])}"></a>' if indent_level == 0 else ""
        ignored = any(
            re.match(ignore, "/".join(path)) is not None
            for ignore in self.ignore_patterns
        )
        if obj.get("deprecated") and not self.show_deprecated:
            # Don't even parse children of deprecated properties
            return output_lines

        if not ignored:
            prefix = "\n###" if indent_level == 0 else f"{indentation}-"

            if has_children and self.collapse_children:
                # Expandable children
                output_lines.extend(
                    [
                        f"{prefix} <details>",
                        "<summary>",
                        markdown.markdown(  # Only HTML is supported for the summary
                            f"{anchor}{name_formatted}{obj_type}",
                        )[
                            3:-4
                        ],  # Remove <p> tags
                        "</summary>\n\n",
                    ],
                )

            else:
                output_lines.append(
                    f"{prefix} {anchor}{name_formatted}{obj_type}",
                )

            description = description_line.strip()
            if indent_level == 0:
                output_lines.extend(["\n", description.strip(":"), "\n\n"])
            else:
                output_lines[-1] += f"{description}\n"
        # Recursively parse subschemas following schema composition keywords
        schema_composition_keyword_map = {
            "allOf": "All of",
            "anyOf": "Any of",
            "oneOf": "One of",
        }
        for key, label in schema_composition_keyword_map.items():
            if key in obj:
                # Only add if the subschema is not ignored
                ignored_child = any(
                    re.match(ignore, "/".join([*path, key])) is not None
                    for ignore in self.ignore_patterns
                )
                if not ignored_child:
                    output_lines.append(
                        f"{indentation_items}- **{label}**\n",
                    )
                for i, child_obj in enumerate(obj[key]):
                    output_lines = self._parse_object(
                        child_obj,
                        path=[*path, key, str(i)],
                        name=None,
                        name_monospace=False,
                        output_lines=output_lines,
                        indent_level=indent_level + 2,
                    )

        # Recursively add items and definitions

        # Recursively add additional child properties
        for extra_props in ["additional", "unevaluated"]:
            property_name = f"{extra_props}Properties"
            if property_name in obj and isinstance(obj[property_name], dict):
                output_lines = self._parse_object(
                    obj[property_name],
                    path=[*path, property_name],
                    name=f"{extra_props.capitalize()} properties",
                    name_monospace=False,
                    output_lines=output_lines,
                    indent_level=indent_level + 1,
                )

        # Recursively add child properties
        for property_name in ["properties", "patternProperties"]:
            if property_name in obj:
                for obj_property_name, property_obj in obj[property_name].items():
                    output_lines = self._parse_object(
                        property_obj,
                        path=[*path, property_name, obj_property_name],
                        name=obj_property_name,
                        output_lines=output_lines,
                        indent_level=indent_level + 1,
                        required=obj_property_name in obj.get("required", []),
                        dependent_required=[
                            k
                            for k, v in obj.get("dependentRequired", {}).items()
                            if obj_property_name in v
                        ],
                    )

        if not ignored and has_children and self.collapse_children:
            output_lines.append(f"\n{indentation_items}</details>\n\n")
        # Add examples
        if self.show_examples in ["all", "properties"]:
            output_lines.extend(
                self._construct_examples(obj, indent_level=indent_level)
            )

        return output_lines

    def parse_schema(
        self,
        schema_object: dict[str, Any],
        fail_on_error_in_defs: bool = True,
    ) -> Sequence[str]:
        """
        Parse JSON Schema object to markdown text.

        Parameters
        ----------
        schema_object: The JSON Schema object to parse.
        fail_on_error_in_defs: If True, the method will raise an error when encountering issues in the
            "definitions" section of the schema. If False, the method will attempt to continue parsing
            despite such errors.

        Returns
        -------
            A list of strings representing the parsed Markdown documentation.
        """
        output_lines = []

        # Add title and description

        if "description" in schema_object:
            output_lines.append(f"*{schema_object['description']}*\n\n")

        # Add items
        if "items" in schema_object:
            output_lines.append(f"#{'#' * (self.header_level + 1)} Items\n\n")
            output_lines.extend(
                self._parse_object(
                    schema_object["items"],
                    path=["items"],
                    name="Items",
                    name_monospace=False,
                ),
            )

        # Add additional/unevaluated properties
        for extra_props in ["additional", "unevaluated"]:
            property_name = f"{extra_props}Properties"
            title_ = f"{extra_props.capitalize()} Properties"
            if property_name in schema_object and isinstance(
                schema_object[property_name], dict
            ):
                output_lines.append(f"#{'#' * (self.header_level + 1)} {title_}\n\n")
                output_lines.extend(
                    self._parse_object(
                        schema_object[property_name],
                        path=[property_name],
                        name=title_,
                        name_monospace=False,
                    ),
                )

        # Add pattern properties
        if "patternProperties" in schema_object:
            output_lines.append(
                f"#{'#' * (self.header_level + 1)} Pattern Properties\n\n"
            )
            for obj_name, obj in schema_object["patternProperties"].items():
                output_lines.extend(
                    self._parse_object(obj, path=["patternProperties"], name=obj_name)
                )

        # Add properties
        output_lines.append(f"#{'#' * (self.header_level + 1)} Properties\n\n")
        output_lines.extend(
            self._parse_object(
                schema_object,
                path=["properties", "ModProfile"],
                name="ModProfile",
                required=True,
                dependent_required=[
                    k
                    for k, v in schema_object.get("dependentRequired", {}).items()
                    if "ModProfile" in v
                ],
            ),
        )

        # Add definitions / $defs
        for name in ["definitions", "$defs"]:
            if name in schema_object:
                output_lines.append(f"#{'#' * (self.header_level + 1)} Definitions\n\n")
                for obj_name, obj in schema_object[name].items():
                    try:
                        output_lines.extend(
                            self._parse_object(
                                obj, path=[name, obj_name], name=obj_name
                            )
                        )
                    except (
                        Exception
                    ) as exception:  # pylint: disable=broad-exception-caught
                        message = f"Error parsing {obj_name} from {name} in schema, usually it occurs when the kind of def is not supported."
                        if fail_on_error_in_defs:
                            raise ValueError(message) from exception
                        print(f"WARN: {message}")

        # Add examples
        if "examples" in schema_object and self.show_examples in ["all", "object"]:
            output_lines.append(f"#{'#' * (self.header_level + 1)} Examples\n\n")
            output_lines.extend(
                self._construct_examples(
                    schema_object, indent_level=0, add_header=False
                )
            )

        return output_lines


def main() -> None:
    """Convert JSON Schema to Markdown documentation."""
    argparser = argparse.ArgumentParser(
        "Convert JSON Schema to Markdown documentation."
    )
    argparser.add_argument(
        "--version", action="version", version=f"%(prog)s {__version__}"
    )
    argparser.add_argument(
        "--pre-commit",
        action="store_true",
        help="Run as pre-commit hook after the generation.",
    )
    argparser.add_argument(
        "--examples-as-yaml",
        action="store_true",
        help="Parse examples in YAML-format instead of JSON.",
    )
    argparser.add_argument(
        "--show-examples",
        choices=["all", "properties", "object"],
        default="all",
        help="Parse examples for only the main object, only properties, or all.",
    )
    argparser.add_argument(
        "--header-level",
        type=int,
        default=0,
        help="Base header level for the generated markdown.",
    )
    argparser.add_argument(
        "--ignore_error_in_defs",
        action="store_false",
        dest="fail_on_error_in_defs",
        default=True,
        help="Ignore errors in definitions.",
    )
    argparser.add_argument("input_json", type=Path, help="Input JSON file.")
    argparser.add_argument("output_markdown", type=Path, help="Output Markdown file.")

    args = argparser.parse_args()

    parser = Parser(
        examples_as_yaml=args.examples_as_yaml,
        show_examples=args.show_examples,
        header_level=args.header_level,
    )
    with args.input_json.open(encoding="utf-8") as input_json:
        output_md = parser.parse_schema(
            json.load(input_json), args.fail_on_error_in_defs
        )

    with args.output_markdown.open("w", encoding="utf-8") as output_markdown:
        output_markdown.writelines(output_md)

    if args.pre_commit:
        subprocess.run(  # pylint: disable=subprocess-run-check # nosec
            [
                "pre-commit",
                "run",
                "--color=never",
                f"--files={args.output_markdown}",
            ],  # noqa: S607,RUF100
            check=False,
        )


if __name__ == "__main__":
    main()
