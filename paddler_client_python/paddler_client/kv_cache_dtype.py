from enum import StrEnum


class KvCacheDtype(StrEnum):
    F32 = "F32"
    F16 = "F16"
    BF16 = "BF16"
    Q8_0 = "Q8_0"
    Q4_0 = "Q4_0"
    Q4_1 = "Q4_1"
    IQ4_NL = "IQ4_NL"
    Q5_0 = "Q5_0"
    Q5_1 = "Q5_1"
