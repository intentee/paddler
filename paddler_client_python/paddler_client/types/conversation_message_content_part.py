from typing import Annotated, Literal

from pydantic import BaseModel, Discriminator, Tag

from paddler_client.types.image_url import ImageUrl


class TextContentPart(BaseModel):
    type: Literal["text"] = "text"
    text: str


class ImageUrlContentPart(BaseModel):
    type: Literal["image_url"] = "image_url"
    image_url: ImageUrl


ConversationMessageContentPart = Annotated[
    Annotated[TextContentPart, Tag("text")]
    | Annotated[ImageUrlContentPart, Tag("image_url")],
    Discriminator("type"),
]
