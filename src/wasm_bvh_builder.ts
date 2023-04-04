import type { BuiltBvh, StagingBuffer } from '../bvh/pkg';
import { _assert, _debugAssert } from './util';

declare module "../bvh/pkg" {
  interface StagingBuffer {
    u32_view(): Uint32Array;
    i32_view(): Int32Array;
    f32_view(): Float32Array;
  }
}

type BvhWasmModule = typeof import('../bvh/pkg');

let _wasm_bvh: BvhWasmModule | undefined;
async function loadBvhWasmModule(): Promise<BvhWasmModule> {
  if (_wasm_bvh) {
    return _wasm_bvh;
  }
  _wasm_bvh = await import('../bvh/pkg');
  patchStagingBuffer(_wasm_bvh);
  return _wasm_bvh;
}

// TODO: lazy load
loadBvhWasmModule();

function patchStagingBuffer(_wasm: BvhWasmModule) {
  if (!_wasm) {
    return;
  }
  _wasm.StagingBuffer.prototype.u32_view = function (): Uint32Array {
    const u8 = this.u8_view() as Uint8Array;
    return new Uint32Array(u8.buffer, u8.byteOffset, Math.floor(u8.byteLength / Uint32Array.BYTES_PER_ELEMENT));
  };

  _wasm.StagingBuffer.prototype.i32_view = function (): Int32Array {
    const u8 = this.u8_view() as Uint8Array;
    return new Int32Array(u8.buffer, u8.byteOffset, Math.floor(u8.byteLength / Int32Array.BYTES_PER_ELEMENT));
  };

  _wasm.StagingBuffer.prototype.f32_view = function (): Float32Array {
    const u8 = this.u8_view() as Uint8Array;
    return new Float32Array(u8.buffer, u8.byteOffset, Math.floor(u8.byteLength / Float32Array.BYTES_PER_ELEMENT));
  };
}

export function allocateStagingBuffer(numBytes: number): StagingBuffer {
  if (!_wasm_bvh) {
    throw 'bvh wasm module not loaded'
  }
  return new _wasm_bvh.StagingBuffer(numBytes);
}

export interface _GPUBufferExtra {
  __staging?: StagingBuffer; // never call {u8, f32, i32}_view() on this staging buffer, the view() function does not consider byte offsets and sizes
  __lastMapped?: {
    mapped: ArrayBuffer;
    offset?: GPUSize64;
  };
}

// see geom.glsl
export const enum GeometryType {
  TRIANGLE = 0,
  AABB = 1,
}
type GeometryDesc = {
  vBufferIndex: number;
  iBufferIndex: number;
  vboOffset: number;
  vboStride: number;
  vioOffset: number;
  vioStride: number;
  owningGeometryType_todo_deprecate: GeometryType;
  owningGeometryFlags: number;
};

function retrieveStagingBuffer(buffer: GPUBuffer): StagingBuffer {
  const s = (buffer as _GPUBufferExtra).__staging;
  if (!s) {
    throw new Error('missing staging buffer');
  }
  return s;
}

// see GeometryDescriptorField::NumFields
const WASM_GEOMETRY_DESCRIPTOR_NUM_I32 = 6;
function buildBlas(desc: GPURayTracingAccelerationContainerDescriptor_bottom, stagingBuffersToFree: Set<StagingBuffer>): BuiltBvh {
  if (!_wasm_bvh) {
    throw 'bvh wasm module not loaded'
  }
  let numTotalPrimitives = 0;
  // [ num_geoms, num_total_primitives, [geom_type, num_primitives, vbuf_id, ibuf_id]+ ]
  const geomBufferIds = allocateStagingBuffer(Int32Array.BYTES_PER_ELEMENT * (2 + WASM_GEOMETRY_DESCRIPTOR_NUM_I32 * desc.geometries.length));
  const geomBufferIds_i32 = geomBufferIds.i32_view();
  geomBufferIds_i32[0] = desc.geometries.length;
  for (let gi = 0; gi < desc.geometries.length; gi++) {
    const geom = desc.geometries[gi];
    const vbuf = retrieveStagingBuffer(geom.type === 'triangles' ? geom.vertex.buffer : geom.aabb.buffer);
    const vbufByteOffset = ((geom.type === 'triangles' ? geom.vertex.offset : geom.aabb.offset) || 0);
    stagingBuffersToFree.add(vbuf);
    const vidx = vbuf.id;
    _assert(vidx !== undefined, '');
    let iidx: number | undefined = -1;
    let ibufByteOffset = 0;
    let np = 1;
    if (geom.type === 'triangles') {
      if (geom.index) {
        const ibuf = retrieveStagingBuffer(geom.index.buffer);
        ibufByteOffset = (geom.index.offset || 0);
        stagingBuffersToFree.add(ibuf);
        iidx = ibuf.id;
        _assert(iidx !== undefined, '');
        _assert(geom.index.size! > 0, '');
        np = Math.floor(geom.index.size! / (3 * Uint32Array.BYTES_PER_ELEMENT)); // 3 indices per primitive
      } else {
        _assert(geom.vertex.size! > 0, '');
        np = Math.floor(geom.vertex.size! / (3 * geom.vertex.stride)); // 3 vertices per primitive
      }
    }
    numTotalPrimitives += np;
    geomBufferIds_i32.set([
      geom.type === 'triangles' ? GeometryType.TRIANGLE : GeometryType.AABB,
      np,
      vidx!,
      vbufByteOffset,
      iidx!,
      ibufByteOffset,
    ], 2 + gi * WASM_GEOMETRY_DESCRIPTOR_NUM_I32);
  }
  geomBufferIds_i32[1] = numTotalPrimitives;

  const serialized = _wasm_bvh.build_blas(geomBufferIds.id);
  geomBufferIds.free();
  return serialized;
}

function _debugPrintTreeAabb(tree: BuiltBvh) {
  const u8 = tree.serialized.u8_view() as Uint8Array;
  _assert(!!u8, 'null built tree');
  const aabb = new Float32Array(u8.buffer, u8.byteOffset, 8); // note the alignment for vec3
  console.debug('tree aabb', aabb);
  // throw 'done'
}

// NOTE: keep in sync with lib.rs::TlasInstanceDescriptorJsInput
const enum TlasInstanceDescriptorField_wordsOffset {
  Mask = 0,
  Flags,
  InstanceId,
  SbtInstanceOffset,
  InstanceCustomIndex,
  BlasEntryIndex,
  BlasGeometryIdOffset,
  BlasAabb,
  Transform4x3 = BlasAabb + 6,

  __numWords = Transform4x3 + 12,
}

const TRANSFORM_IDENTITY_COL_MAJOR_4x3 = new Float32Array([
  1, 0, 0,
  0, 1, 0,
  0, 0, 1,
  0, 0, 0
]);

export class Tlas {
  private _bufferBvhTree: [GPUBuffer, GPUBuffer] | undefined;
  constructor(private readonly _descriptor: GPURayTracingAccelerationContainerDescriptor_top) {
  }

  built(): boolean {
    return !!this._bufferBvhTree;
  }

  allUniqueGeomBuffer(): [GeometryDesc[], Map<GPUBuffer, number>] {
    const buffers = new Map<GPUBuffer, number>();
    const descriptors: GeometryDesc[] = [];
    const visitedBlas: Set<GPURayTracingAccelerationContainerDescriptor_bottom> = new Set();
    for (const inst of this._descriptor.instances) {
      if (visitedBlas.has(inst.blas)) {
        continue;
      }
      visitedBlas.add(inst.blas);
      for (const geom of inst.blas.geometries) {
        if (geom.type === 'triangles') {
          let vidx = buffers.get(geom.vertex.buffer);
          if (vidx === undefined) {
            vidx = buffers.size;
            buffers.set(geom.vertex.buffer, vidx);
          }

          let iidx = -1;
          if (geom.index) {
            let idx = buffers.get(geom.index.buffer);
            if (idx === undefined) {
              idx = buffers.size;
              buffers.set(geom.index.buffer, idx);
            }
            iidx = idx;
          }

          descriptors.push({
            vBufferIndex: vidx,
            iBufferIndex: iidx,
            vboOffset: geom.vertex.offset!,
            vioOffset: geom.index?.offset!,
            vboStride: geom.vertex.stride,
            vioStride: 12,
            owningGeometryType_todo_deprecate: GeometryType.TRIANGLE,
            owningGeometryFlags: 0,
          });
        } else {
          let vidx = buffers.get(geom.aabb.buffer);
          if (vidx === undefined) {
            vidx = buffers.size;
            buffers.set(geom.aabb.buffer, vidx);
          }

          descriptors.push({
            vBufferIndex: vidx,
            iBufferIndex: -1,
            vboOffset: geom.aabb.offset!,
            vioOffset: 0,
            vboStride: geom.aabb.stride!,
            vioStride: 0,
            owningGeometryType_todo_deprecate: GeometryType.AABB,
            owningGeometryFlags: 0,
          });
        }
      }
    }

    console.debug('total buffers', buffers.size);
    return [descriptors, buffers];
  }

  //! Note that in current setup geometries are not recommended to be shared between BLASes
  //! it's better to build a BLAS for the geometry and share in TLAS
  //! this means if same geom descriptor appears in two BLASes, it's return twice in the array
  //! result in two buffer handles?

  //! This assumes all buffers are used for the whole range
  allGeomBuffersInOrder(): GPUBuffer[] {
    const buffers = Array.from(this.allUniqueGeomBuffer()[1].entries());
    buffers.sort((a, b) => a[1] - b[1]);
    return buffers.map(b => b[0]);
  }

  getBvhTreeNodesBuffers(): [GPUBuffer, GPUBuffer] {
    if (!this._bufferBvhTree) {
      throw 'getBvhTreeNodesBuffers but not built'
    }
    return this._bufferBvhTree;
  }

  build(device: GPUDevice) {
    if (this._bufferBvhTree) {
      return;
    }
    if (!_wasm_bvh) {
      throw 'bvh wasm module not loaded'
    }
    let blasGPUBuffer: GPUBuffer | undefined;
    // TODO: separate blas build and tlas build
    const builtBlasTreesInfo: Map<GPURayTracingAccelerationContainerDescriptor_bottom, [number/* blas_entry_index */, number/* blas_geometry_id_offset */, Float32Array/*aabb*/]> = new Map();
    {
      const stagingBuffersToFree: Set<StagingBuffer> = new Set();
      let blasTotalBufferSize = 0;
      const trees: StagingBuffer[] = [];
      let blas_geometry_id_offset = 0;
      let blas_entry_index = 0; // TODO: if tlas is in the front, the offset for the first blas is unknown until tlas finishes building
      for (let i = 0; i < this._descriptor.instances.length; i++) {
        const inst = this._descriptor.instances[i];
        if (builtBlasTreesInfo.has(inst.blas)) {
          continue;
        }
        const builtBlas = buildBlas(inst.blas, stagingBuffersToFree);
        const u8 = builtBlas.serialized.u8_view() as Uint8Array;
        _assert(!!u8, 'null built blas tree');
        const aabb = new Float32Array(6);
        aabb.set(new Float32Array(u8.buffer, u8.byteOffset, 3));
        aabb.set(new Float32Array(u8.buffer, u8.byteOffset + 16, 3), 3); // note the vec3 alignment
        builtBlasTreesInfo.set(inst.blas, [blas_entry_index, blas_geometry_id_offset, aabb]);

        blas_entry_index += builtBlas.num_nodes;
        blas_geometry_id_offset += inst.blas.geometries.length;
        blasTotalBufferSize += u8.byteLength; // TODO: should verify element size
        trees.push(builtBlas.serialized);
      }

      blasGPUBuffer = device.createBuffer({
        size: blasTotalBufferSize,
        usage: GPUBufferUsage.STORAGE,
        mappedAtCreation: true,
      });
      const buf = blasGPUBuffer.getMappedRange();
      // same order as entry_index, geom_id_offset calculation
      let byteOffset = 0;
      for (const bt of trees) {
        const u8 = bt.u8_view() as Uint8Array;
        new Uint8Array(buf, byteOffset).set(u8);
        byteOffset += u8.byteLength;
        bt.free();
      }
      // console.debug('@@gpu_blas', new Uint32Array(buf, 0, buf.byteLength / 4).toString());
      blasGPUBuffer.unmap();
      // TODO: make sure no other places can reference the same buffer (w/ different offsets)
      for (const b of stagingBuffersToFree) {
        b.free();
      }
    }
    console.log('serialized # unique blas tree: ', builtBlasTreesInfo.size);

    // TODO: instanceCustomIndex could be i32
    const tlasInstanceDescriptors = allocateStagingBuffer(
      Uint32Array.BYTES_PER_ELEMENT * (1 + TlasInstanceDescriptorField_wordsOffset.__numWords * this._descriptor.instances.length));
    const tlasInstanceDescriptors_u32 = tlasInstanceDescriptors.u32_view();
    tlasInstanceDescriptors_u32[0] = this._descriptor.instances.length;
    for (let i = 0; i < this._descriptor.instances.length; i++) {
      const inst = this._descriptor.instances[i];
      const builtBlas = builtBlasTreesInfo.get(inst.blas);
      if (!builtBlas) {
        throw 'built blas tree not found'
      }

      const wordStart = 1 + i * TlasInstanceDescriptorField_wordsOffset.__numWords;
      tlasInstanceDescriptors_u32.set([
        0xff, // inst.mask,
        0, // inst.flags
        i,
        inst.instanceSBTRecordOffset,
        (inst.instanceCustomIndex ?? -1),
        builtBlas[0], // blas_entry_index,
        builtBlas[1],//blas_geometry_id_offset,
      ], wordStart);

      new Float32Array(
        tlasInstanceDescriptors_u32.buffer,
        tlasInstanceDescriptors_u32.byteOffset +
        Uint32Array.BYTES_PER_ELEMENT *
        (wordStart + TlasInstanceDescriptorField_wordsOffset.BlasAabb)
      ).set(builtBlas[2]);

      new Float32Array(
        tlasInstanceDescriptors_u32.buffer,
        tlasInstanceDescriptors_u32.byteOffset +
        Uint32Array.BYTES_PER_ELEMENT *
        (wordStart + TlasInstanceDescriptorField_wordsOffset.Transform4x3)
      ).set(inst.transformMatrix || TRANSFORM_IDENTITY_COL_MAJOR_4x3);
    }

    let tlasGPUBuffer: GPUBuffer | undefined;
    {
      const builtTlas = _wasm_bvh.build_tlas(tlasInstanceDescriptors.id);
      tlasInstanceDescriptors.free();
      _debugPrintTreeAabb(builtTlas);
      const tlas_u8 = builtTlas.serialized.u8_view() as Uint8Array;

      tlasGPUBuffer = device.createBuffer({
        size: tlas_u8.byteLength,
        usage: GPUBufferUsage.STORAGE,
        mappedAtCreation: true,
      });
      const buf = tlasGPUBuffer.getMappedRange();
      new Uint8Array(buf, 0).set(tlas_u8);
      // console.debug('@@gpu_tlas', new Uint32Array(tlas_u8.buffer, tlas_u8.byteOffset, tlas_u8.byteLength / 4));
      tlasGPUBuffer.unmap();
      builtTlas.serialized.free();
    }

    this._bufferBvhTree = [tlasGPUBuffer, blasGPUBuffer];
    return this._bufferBvhTree;
  }
}
