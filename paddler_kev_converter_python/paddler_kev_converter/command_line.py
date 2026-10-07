from argparse import ArgumentParser, Namespace
from pathlib import Path

from paddler_kev_converter.convertible_kev_checkpoint import ConvertibleKevCheckpoint
from paddler_kev_converter.malformed_pointer_head import (
    PointerHeadDefect,
    write_malformed_pointer_head,
)
from paddler_kev_converter.parity_reference import write_parity_reference
from paddler_kev_converter.synthetic_pointer_head import synthetic_pointer_head
from paddler_kev_converter.tokenizer_reference import write_tokenizer_reference
from paddler_kev_converter.typesafe_reference import write_typesafe_reference


def _convert(parsed_arguments: Namespace) -> None:
    checkpoint = ConvertibleKevCheckpoint.open(parsed_arguments.checkpoint)
    checkpoint.write_backbone(parsed_arguments.backbone_directory)
    checkpoint.pointer_head().write_gguf(parsed_arguments.pointer_head)


def _write_malformed_pointer_head(parsed_arguments: Namespace) -> None:
    write_malformed_pointer_head(
        parsed_arguments.defect,
        parsed_arguments.hidden_size,
        parsed_arguments.output,
    )


def _write_parity_reference(parsed_arguments: Namespace) -> None:
    write_parity_reference(parsed_arguments.checkpoint, parsed_arguments.output)


def _write_synthetic_pointer_head(parsed_arguments: Namespace) -> None:
    synthetic_pointer_head(parsed_arguments.hidden_size).write_gguf(
        parsed_arguments.output
    )


def _write_tokenizer_reference(parsed_arguments: Namespace) -> None:
    write_tokenizer_reference(parsed_arguments.checkpoint, parsed_arguments.output)


def _write_typesafe_reference(parsed_arguments: Namespace) -> None:
    write_typesafe_reference(parsed_arguments.output)


def main(arguments: list[str] | None = None) -> None:
    parser = ArgumentParser(prog="paddler-kev-converter")
    commands = parser.add_subparsers(required=True)

    convert = commands.add_parser("convert")
    convert.add_argument("--checkpoint", required=True)
    convert.add_argument("--backbone-directory", required=True, type=Path)
    convert.add_argument("--pointer-head", required=True, type=Path)
    convert.set_defaults(handle=_convert)

    malformed = commands.add_parser("malformed-pointer-head")
    malformed.add_argument("--defect", required=True, type=PointerHeadDefect)
    malformed.add_argument("--hidden-size", required=True, type=int)
    malformed.add_argument("--output", required=True, type=Path)
    malformed.set_defaults(handle=_write_malformed_pointer_head)

    parity = commands.add_parser("parity-reference")
    parity.add_argument("--checkpoint", required=True)
    parity.add_argument("--output", required=True, type=Path)
    parity.set_defaults(handle=_write_parity_reference)

    synthetic = commands.add_parser("synthetic-pointer-head")
    synthetic.add_argument("--hidden-size", required=True, type=int)
    synthetic.add_argument("--output", required=True, type=Path)
    synthetic.set_defaults(handle=_write_synthetic_pointer_head)

    tokenizer = commands.add_parser("tokenizer-reference")
    tokenizer.add_argument("--checkpoint", required=True)
    tokenizer.add_argument("--output", required=True, type=Path)
    tokenizer.set_defaults(handle=_write_tokenizer_reference)

    typesafe = commands.add_parser("typesafe-reference")
    typesafe.add_argument("--output", required=True, type=Path)
    typesafe.set_defaults(handle=_write_typesafe_reference)

    parsed_arguments = parser.parse_args(arguments)
    parsed_arguments.handle(parsed_arguments)
