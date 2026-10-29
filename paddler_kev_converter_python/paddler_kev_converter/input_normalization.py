from enum import StrEnum


class InputNormalization(StrEnum):
    NFC = "NFC"
    NFD = "NFD"
    NFKC = "NFKC"
    NFKD = "NFKD"
    NONE = "None"
