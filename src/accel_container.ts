import { Std140Block } from './blocklayout';
import { GPURayTracingPipelineImpl } from './pipeline';
import { _debugAssert } from './util';
import { Tlas } from './wasm_bvh_builder';

// start&stride for RayMiss, Hit, Callable, only start for RayGen.
const RT_UNIFORM_PARAMS_NUM_BIND_LOCATIONS = 1;// (2 * 4);

const BP_RT_UNIFORM_PARAMS = 0;
const BP_SBT = BP_RT_UNIFORM_PARAMS + RT_UNIFORM_PARAMS_NUM_BIND_LOCATIONS;
const BP_TLAS_BVH_TREE_NODES = BP_SBT + 1;
const BP_BLASES_BVH_TREE_NODES = BP_TLAS_BVH_TREE_NODES + 1;
const BP_GEOM_BUFFERS_START = BP_BLASES_BVH_TREE_NODES + 1;

export class GPURayTracingAccelerationContainer_top_Impl implements GPURayTracingAccelerationContainer_top {
  private _tlas: Tlas;
  private _bindGroup: GPUBindGroup | undefined;
  private _rtUniformParams: GPUBuffer | undefined;

  constructor(private readonly _descriptor: GPURayTracingAccelerationContainerDescriptor_top) {
    this._tlas = new Tlas(this._descriptor);
  }

  hostBuild(device: GPUDevice) {
    this._tlas.build(device);
  }

  getBvhGeometryBuffersAndDescriptors() {
    return this._tlas.allUniqueGeomBuffer();
  }

  createFinalRtBindGroup(device: GPUDevice,
    pipeline: GPURayTracingPipelineImpl,
    sbt: GPUShaderBindingTable,
    numWorkGroups: readonly [number, number, number],
  ): GPUBindGroup {
    if (!this._tlas.built()) {
      throw 'createFinalRtBindGroup but TLAS is not built yet'
    }
    // TODO: this assumes nothing ever changes
    if (this._bindGroup) {
      return this._bindGroup;
    }
    {
      // std140
      const layout = new Std140Block();
      for (const uint32 of [
        sbt.rayGen.start,
        sbt.rayMiss.start, sbt.rayMiss.stride,
        sbt.rayHit.start, sbt.rayHit.stride,
        sbt.callable.start, sbt.callable.stride,
      ]) {
        layout.addUint32(uint32);
      }
      layout.addUvec3(numWorkGroups[0], numWorkGroups[1], numWorkGroups[2]);
      const data8 = layout.getFinalUint8Array();

      this._rtUniformParams = device.createBuffer({
        size: data8.byteLength,
        usage: GPUBufferUsage.UNIFORM,
        mappedAtCreation: true,
      });
      new Uint8Array(this._rtUniformParams.getMappedRange()).set(data8);
      this._rtUniformParams.unmap();
    }

    const [tlasBuffer, blasesBuffer] = this._tlas.getBvhTreeNodesBuffers();

    const entries: GPUBindGroupEntry[] = [{
      binding: BP_RT_UNIFORM_PARAMS,
      resource: {
        buffer: this._rtUniformParams,
      },
    }, {
      binding: BP_SBT,
      resource: {
        buffer: sbt.buffer,
      },
    }, {
      binding: BP_TLAS_BVH_TREE_NODES,
      resource: {
        buffer: tlasBuffer,
      },
    }, {
      binding: BP_BLASES_BVH_TREE_NODES,
      resource: {
        buffer: blasesBuffer,
      },
    },];

    entries.push(...this._tlas.allGeomBuffersInOrder().map((b, i) => ({
      binding: BP_GEOM_BUFFERS_START + i,
      resource: {
        buffer: b,
      },
    })));

    // _debugAssert(!!this._bindGroupLayout, 'missing _bindGroupLayout');
    this._bindGroup = device.createBindGroup({
      // layout: this._bindGroupLayout!,
      //! prefer the layout set in pipeline
      layout: pipeline.getBindGroupLayout(pipeline.getInternalBindSet()),
      entries,
    })
    // _debugAssert(!!this._rtUniformParams, 'missing _rtUniformParams');
    // await this._rtUniformParams!.mapAsync(GPUMapMode.WRITE);
    return this._bindGroup;
  }

  destroy() {
    // TODO: free all host WASM buffers
  }
}
