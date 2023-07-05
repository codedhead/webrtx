import { _GPUShaderStageRTX } from "./types";
import { _debugAssert } from "./util";

// see common.glsl
const WEBRTX_SHADER_UNUSED = 0xff;

export class GPURayTracingPipelineImpl implements GPURayTracingPipeline {
  constructor(
    private readonly _descriptor: GPURayTracingPipelineDescriptor,
    private readonly _todo_decouple_tlas: GPURayTracingAccelerationContainer_top,
    readonly comp: GPUComputePipeline,
    private readonly _rtInternalResourcesBindingSet: number) {
  }

  // TODO: do not expose these functions
  getInternalBindSet() {
    return this._rtInternalResourcesBindingSet;
  }

  todo_drop_getBoundAccelerationStructure(): GPURayTracingAccelerationContainer_top {
    return this._todo_decouple_tlas;
  }

  getBindGroupLayout(
    index: number
  ): GPUBindGroupLayout {
    return this.comp.getBindGroupLayout(index);
  }

  getShaderGroupHandles(first: number, count: number): ShaderGroupHandle[] {
    if (first < 0 || first + count > this._descriptor.groups.length) {
      throw 'getShaderGroupHandles out of bound'
    }

    const shaderStageHandle: number[] = [];
    const _stages: Map<_GPUShaderStageRTX, number> = new Map();
    for (let i = 0; i < this._descriptor.stages.length; i++) {
      const stg = this._descriptor.stages[i];
      let shaders_in_stage = (_stages.get(stg.stage) || 0);
      const indexWithinStage = shaders_in_stage; // this is the shader handle!
      shaderStageHandle.push(indexWithinStage);
      _stages.set(stg.stage, shaders_in_stage + 1);
    }

    const res: ShaderGroupHandle[] = [];
    for (let i = 0; i < count; i++) {
      const g = this._descriptor.groups[first + i];
      let intersect = WEBRTX_SHADER_UNUSED;
      switch (g.type) {
        case 'general': {
          const stage = this._descriptor.stages[g.generalIndex].stage;
          _debugAssert((stage === GPUShaderStageRTX.RAY_GENERATION
            || stage === GPUShaderStageRTX.RAY_MISS), // TODO: callable
            'generalIndex does not refer to an rgen shader');
          res.push(shaderStageHandle[g.generalIndex] & 0xff);
          break;
        }
        case 'procedural-hit-group':
          _debugAssert(this._descriptor.stages[g.intersectionIndex].stage === GPUShaderStageRTX.RAY_INTERSECTION,
            'intersectionIndex does not refer to an rint shader');
          intersect = shaderStageHandle[g.intersectionIndex] & 0xff;
        // fall through
        case 'triangles-hit-group': {
          let ahit = WEBRTX_SHADER_UNUSED, chit = WEBRTX_SHADER_UNUSED;
          if (g.anyHitIndex !== undefined) {
            _debugAssert(this._descriptor.stages[g.anyHitIndex].stage === GPUShaderStageRTX.RAY_ANY_HIT,
              'anyHitIndex does not refer to an rahit shader');
            ahit = shaderStageHandle[g.anyHitIndex] & 0xff;
          }
          if (g.closestHitIndex !== undefined) {
            _debugAssert(this._descriptor.stages[g.closestHitIndex].stage === GPUShaderStageRTX.RAY_CLOSEST_HIT,
              'anyHitIndex does not refer to an rahit shader');
            chit = shaderStageHandle[g.closestHitIndex] & 0xff;
          }
          res.push((ahit << 16) | (chit << 8) | intersect);
          break;
        }
      }
    }
    return res;
  }
}
