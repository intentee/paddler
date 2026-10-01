from pydantic import ValidationError


def validation_error_cause(validation_error: ValidationError) -> object:
    [error_details] = validation_error.errors()

    return error_details["ctx"]["error"]
