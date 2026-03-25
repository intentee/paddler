from enum import StrEnum


class PoolingType(StrEnum):
    UNSPECIFIED = "Unspecified"
    NONE = "None"
    MEAN = "Mean"
    CLS = "Cls"
    LAST = "Last"
    RANK = "Rank"
