from gguf import GGUFReader


def tensor_shapes(reader: GGUFReader) -> dict[str, tuple[int, ...]]:
    return {
        tensor.name: tuple(int(dimension) for dimension in tensor.shape)
        for tensor in reader.tensors
    }
