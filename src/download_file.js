export function download_file(filename, content) {
  console.log({ filename, content });
  const element = document.createElement("a");
  element.setAttribute(
    "href",
    "data:text/plain;charset=utf-8," + encodeURIComponent(content)
  );
  element.setAttribute("download", filename);
  element.style.display = "none";

  document.body.appendChild(element);

  element.click();

  document.body.removeChild(element);
}
