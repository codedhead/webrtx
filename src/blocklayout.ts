import { LITTLE_ENDIAN } from "./constants";
import { alignTo, _debugAssert } from "./util";

type vec3 = [number, number, number] | Float32Array;

export class Std140Block {
  curStructAlignment: number = 4;
  private _buf: ArrayBuffer;
  private _dv: DataView;
  private _bufOffset = 0;
  private _namedLocation = new Map<string, number>();

  constructor(initialSize?: number) {
    initialSize = initialSize || 4;
    this._buf = new ArrayBuffer(initialSize);
    this._dv = new DataView(this._buf);
  }

  reset() {
    this._bufOffset = 0;
    this.curStructAlignment = 4;
    this._namedLocation.clear();
  }

  private _ensureBufferSize(wantMoreByteSize: number) {
    const targetSize = this._bufOffset + wantMoreByteSize;
    if (targetSize > this._buf.byteLength) {
      const newbuf = new ArrayBuffer(Math.max(this._buf.byteLength * 2, targetSize));
      new Uint8Array(newbuf).set(new Uint8Array(this._buf));
      this._buf = newbuf;
      this._dv = new DataView(this._buf);
    }
  }

  getLocation(name: string): number {
    const location = this._namedLocation.get(name);
    if (location === undefined) {
      throw 'named location not fond: ' + name
    }
    return location;
  }

  addUint32(v: number, name?: string) {
    this._ensureBufferSize(4);
    this._dv.setUint32(this._bufOffset, v, LITTLE_ENDIAN);
    if (name) {
      this._namedLocation.set(name, this._bufOffset);
    }
    this._bufOffset += 4;
  }

  addVec2(x: number, y: number) {
    const sizeAndAlign = 4 * 2;
    this._bufOffset = alignTo(this._bufOffset, sizeAndAlign);
    this._ensureBufferSize(sizeAndAlign);
    this._dv.setFloat32(this._bufOffset, x, LITTLE_ENDIAN);
    this._dv.setFloat32(this._bufOffset + 4, y, LITTLE_ENDIAN);
    this._bufOffset += sizeAndAlign;
    this.curStructAlignment = Math.max(this.curStructAlignment, sizeAndAlign);
  }

  addVec3(xyz: vec3) {
    this.addVec4(xyz[0], xyz[1], xyz[2], 0);
  }

  addVec4(x: number, y: number, z: number, w: number) {
    const sizeAndAlign = 16;
    this._bufOffset = alignTo(this._bufOffset, sizeAndAlign);
    this._ensureBufferSize(sizeAndAlign);
    this._dv.setFloat32(this._bufOffset, x, LITTLE_ENDIAN);
    this._dv.setFloat32(this._bufOffset + 4, y, LITTLE_ENDIAN);
    this._dv.setFloat32(this._bufOffset + 8, z, LITTLE_ENDIAN);
    this._dv.setFloat32(this._bufOffset + 12, w, LITTLE_ENDIAN);
    this._bufOffset += sizeAndAlign;
    this.curStructAlignment = Math.max(this.curStructAlignment, sizeAndAlign);
  }

  addUvec3(x: number, y: number, z: number) {
    this.addUvec4(x, y, z, 0);
  }

  addUvec4(x: number, y: number, z: number, w: number) {
    const align = 16;
    this._bufOffset = alignTo(this._bufOffset, align);
    this._ensureBufferSize(16);
    this._dv.setUint32(this._bufOffset, x, LITTLE_ENDIAN);
    this._dv.setUint32(this._bufOffset + 4, y, LITTLE_ENDIAN);
    this._dv.setUint32(this._bufOffset + 8, z, LITTLE_ENDIAN);
    this._dv.setUint32(this._bufOffset + 12, w, LITTLE_ENDIAN);
    this._bufOffset += 16;
    this.curStructAlignment = Math.max(this.curStructAlignment, align);
  }

  getFinalUint8Array(): Uint8Array {
    return new Uint8Array(this._buf, 0, this._bufOffset);
  }
}

export class Std430Block {
  curStructAlignment: number = 4;
  private _buf = new ArrayBuffer(4);
  private _dv = new DataView(this._buf);
  private _bufOffset = 0;

  private _ensureBufferSize(wantMoreByteSize: number) {
    const targetSize = this._bufOffset + wantMoreByteSize;
    if (targetSize > this._buf.byteLength) {
      const newbuf = new ArrayBuffer(Math.max(this._buf.byteLength * 2, targetSize));
      new Uint8Array(newbuf).set(new Uint8Array(this._buf));
      this._buf = newbuf;
      this._dv = new DataView(this._buf);
    }
  }

  addUint32(v: number) {
    this._ensureBufferSize(4);
    this._dv.setUint32(this._bufOffset, v, LITTLE_ENDIAN);
    this._bufOffset += 4;
  }
  addVec2(x: number, y: number) {
    const sizeAndAlign = 4 * 2;
    this._bufOffset = alignTo(this._bufOffset, sizeAndAlign);
    this._ensureBufferSize(sizeAndAlign);
    this._dv.setFloat32(this._bufOffset, x, LITTLE_ENDIAN);
    this._dv.setFloat32(this._bufOffset + 4, y, LITTLE_ENDIAN);
    this._bufOffset += sizeAndAlign;
    this.curStructAlignment = Math.max(this.curStructAlignment, sizeAndAlign);
  }

  addVec3(xyz: vec3) {
    this.addVec4(xyz[0], xyz[1], xyz[2], 0);
  }

  addVec4(x: number, y: number, z: number, w: number) {
    const sizeAndAlign = 16;
    this._bufOffset = alignTo(this._bufOffset, sizeAndAlign);
    this._ensureBufferSize(sizeAndAlign);
    this._dv.setFloat32(this._bufOffset, x, LITTLE_ENDIAN);
    this._dv.setFloat32(this._bufOffset + 4, y, LITTLE_ENDIAN);
    this._dv.setFloat32(this._bufOffset + 8, z, LITTLE_ENDIAN);
    this._dv.setFloat32(this._bufOffset + 12, w, LITTLE_ENDIAN);
    this._bufOffset += sizeAndAlign;
    this.curStructAlignment = Math.max(this.curStructAlignment, sizeAndAlign);
  }

  addUvec3(x: number, y: number, z: number) {
    this.addUvec4(x, y, z, 0);
  }

  addUvec4(x: number, y: number, z: number, w: number) {
    const align = 16;
    this._bufOffset = alignTo(this._bufOffset, align);
    this._ensureBufferSize(16);
    this._dv.setUint32(this._bufOffset, x, LITTLE_ENDIAN);
    this._dv.setUint32(this._bufOffset + 4, y, LITTLE_ENDIAN);
    this._dv.setUint32(this._bufOffset + 8, z, LITTLE_ENDIAN);
    this._dv.setUint32(this._bufOffset + 12, w, LITTLE_ENDIAN);
    this._bufOffset += 16;
    this.curStructAlignment = Math.max(this.curStructAlignment, align);
  }

  addColumnMajorMat3(mat: [vec3, vec3, vec3]) {
    const COLS = 3, ROWS = 3;
    const elementSizeAndAlign = 4 * ROWS; // R component
    const arraySize = elementSizeAndAlign * COLS;
    this._bufOffset = alignTo(this._bufOffset, elementSizeAndAlign);
    this._ensureBufferSize(arraySize);
    for (let n = 0; n < COLS; n++) {
      for (let m = 0; m < ROWS; m++) {
        this._dv.setFloat32(this._bufOffset + 4 * (n * ROWS + m), mat[n][m], LITTLE_ENDIAN);
      }
    }
    this._bufOffset += arraySize;
    this.curStructAlignment = Math.max(this.curStructAlignment, elementSizeAndAlign);
  }

  getFinalUint8Array(): Uint8Array {
    return new Uint8Array(this._buf, 0, this._bufOffset);
  }
}