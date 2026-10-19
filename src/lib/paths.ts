// Path helpers that work on both slash styles (the app only runs on Windows,
// but dropped paths can use either).
export const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;
export const folderOf = (path: string) =>
  path.slice(0, Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/')) + 1);
export const stemOf = (path: string) => fileName(path).replace(/\.pdf$/i, '');

const IMAGE_PATTERN = /\.(jpe?g|jfif|png|bmp|gif|tiff?|webp)$/i;
export const isImage = (path: string) => IMAGE_PATTERN.test(path);
/** File name without its extension, for any file type. */
export const stemOfAny = (path: string) => fileName(path).replace(/\.[^.]+$/, '');
export const IMAGE_EXTENSIONS = ['jpg', 'jpeg', 'jfif', 'png', 'bmp', 'gif', 'tif', 'tiff', 'webp'];
