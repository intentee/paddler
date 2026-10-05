from paddler_client.pooling_type import PoolingType


def test_pooling_type_uses_the_wire_names() -> None:
    assert PoolingType.MEAN.value == "Mean"
    assert PoolingType.CLS.value == "Cls"
    assert PoolingType.LAST.value == "Last"
    assert PoolingType.NONE.value == "None"
    assert PoolingType.RANK.value == "Rank"
    assert PoolingType.UNSPECIFIED.value == "Unspecified"
