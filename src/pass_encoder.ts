import { GPURayTracingAccelerationContainer_top_Impl } from "./accel_container";
import { GPURayTracingPipelineImpl } from "./pipeline";

const COMP_SHADER_WORKGROUP_SIZE = [8, 8, 1] as const;

// TODO: unbind when destroyed
export interface GPUBindGroupWithAccelerationContainer {
  __accel_container?: GPURayTracingAccelerationContainer_top_Impl;
}

export default class GPURayTracingPassEncoderImpl implements GPURayTracingPassEncoder {
  private _pipeline: GPURayTracingPipelineImpl | undefined;
  private _boundAS: GPURayTracingAccelerationContainer_top_Impl | undefined;

  constructor(private readonly _comp: GPUComputePassEncoder) {
  }

  setPipeline(pipeline: GPURayTracingPipelineImpl): void {
    this._pipeline = pipeline;
    this._comp.setPipeline(pipeline.comp);
  }

  setBindGroup(index: GPUIndex32, bindGroup: GPUBindGroup) {
    const theAS = (bindGroup as GPUBindGroupWithAccelerationContainer).__accel_container;
    //! // TODO: detect if there is existing bind group containing AS
    if (theAS) {
      if (this._boundAS) {
        throw 'already bound AS'
      }
      this._boundAS = theAS;
    }
    this._comp.setBindGroup(index, bindGroup);
  }

  traceRays(
    device: GPUDevice,
    sbt: GPUShaderBindingTable,
    width: GPUSize32,
    height: GPUSize32,
    depth?: GPUSize32,
  ): void {
    if (!this._boundAS) {
      throw 'missing acceleration structure'
    }
    if (!this._pipeline) {
      throw 'no ray tracing pipeline is set'
    }
    if (this._boundAS !== this._pipeline.todo_drop_getBoundAccelerationStructure()) {
      throw 'bound acceleration structure not matching the one used for building the ray tracing pipeline'
    }
    depth = depth || 1;
    if ((width % COMP_SHADER_WORKGROUP_SIZE[0])
      || (height % COMP_SHADER_WORKGROUP_SIZE[1])
      || (depth % COMP_SHADER_WORKGROUP_SIZE[2])) {
      throw `width(${width}), height(${height}), depth(${depth}) must be divisible by workgroup size`
    }
    const numWorkGroups = [
      width / COMP_SHADER_WORKGROUP_SIZE[0],
      height / COMP_SHADER_WORKGROUP_SIZE[1],
      depth / COMP_SHADER_WORKGROUP_SIZE[2]] as const;
    this._comp.setBindGroup(this._pipeline.getInternalBindSet(),
      // TODO: upload all offsets here as uniform/push constants
      this._boundAS.createFinalRtBindGroup(device, this._pipeline, sbt, numWorkGroups));
    this._comp.dispatchWorkgroups(...numWorkGroups);
  }

  end(): void {
    this._comp.end();
  }
}
