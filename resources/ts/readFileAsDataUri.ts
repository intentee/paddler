export function readFileAsDataUri(file: File): Promise<string> {
  return new Promise(function (resolve, reject) {
    const fileReader = new FileReader();

    fileReader.addEventListener("load", function () {
      if (typeof fileReader.result === "string") {
        resolve(fileReader.result);
      } else {
        reject(new Error("FileReader did not produce a string result"));
      }
    });

    fileReader.addEventListener("error", function () {
      reject(fileReader.error ?? new Error("FileReader encountered an error"));
    });

    fileReader.readAsDataURL(file);
  });
}
