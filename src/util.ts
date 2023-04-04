
export function alignTo(x: number, align: number): number {
  return Math.floor((x + align - 1) / align) * align;
}

export function _debugAssert(x: boolean, msg: string) {
  if (!x) {
    throw msg
  }
}

export function _assert(x: boolean, msg: string) {
  if (!x) {
    throw msg
  }
}