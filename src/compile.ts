const glslRtEngineCode = require('./glsl/main.comp'); // TODO: no require
import { GPURayTracingAccelerationContainer_top_Impl } from './accel_container';
import { _GPUShaderStageRTX } from './types';
import { nagaModule, glslangModule, glslTranspilerModule } from './wasm_modules';

const GLOBAL_NAME__HIT_ATTRIBUTES_MAX_WORDS = "_CRT_HIT_ATTRIBUTES_MAX_WORDS";
const GLOBAL_NAME__USER_NEXT_UNUSED_BIND_SET = "_CRT_USER_NEXT_UNUSED_BIND_SET";
const MIN_HIT_ATTRIBUTES_WORDS = 5; // barry coords + normal

const glslRtPrelude = `#version 450
#pragma shader_stage(compute)
`;

function canonicalShaderStageEntryName(stg: _GPUShaderStageRTX, i: number): string {
  let prefix = '';
  switch (stg) {
    case GPUShaderStageRTX.RAY_GENERATION:
      prefix = '_crt_user_rgen_';
      break;
    case GPUShaderStageRTX.RAY_ANY_HIT:
      prefix = '_crt_user_rahit_';
      break;
    case GPUShaderStageRTX.RAY_CLOSEST_HIT:
      prefix = '_crt_user_rchit_';
      break;
    case GPUShaderStageRTX.RAY_MISS:
      prefix = '_crt_user_rmiss_';
      break;
    case GPUShaderStageRTX.RAY_INTERSECTION:
      prefix = '_crt_user_rint_';
      break;
    default:
      throw 'unknown shader stage ' + stg;
  }
  return prefix + i;
}

export async function aggregateAndCompileShaders(device: GPUDevice, descriptor: GPURayTracingPipelineDescriptor, tlas: GPURayTracingAccelerationContainer_top): Promise<[GPUShaderModule, number]> {
  // TODO: parse and combine shaders here
  const [glslang, glslTranspiler, naga] = await Promise.all([glslangModule(), glslTranspilerModule(), nagaModule()]);
  if (!descriptor.stages.length) {
    throw 'no stages defined for ray tracing pipeline'
  }

  let maxUsedBindSet = -1;

  // TODO: move out?
  const USER_FUNCTIONS_TABLE_NAME = {
    [GPUShaderStageRTX.RAY_GENERATION]: '_CRT_USER_RGEN_TABLE',
    [GPUShaderStageRTX.RAY_MISS]: '_CRT_USER_RMISS_TABLE',
    [GPUShaderStageRTX.RAY_INTERSECTION]: '_CRT_USER_RINT_TABLE',
    [GPUShaderStageRTX.RAY_CLOSEST_HIT]: '_CRT_USER_RCHIT_TABLE',
    [GPUShaderStageRTX.RAY_ANY_HIT]: '_CRT_USER_RAHIT_TABLE',
  };
  const SHADER_STAGE_NAME = {
    [GPUShaderStageRTX.RAY_GENERATION]: 'rgen',
    [GPUShaderStageRTX.RAY_MISS]: 'rmiss',
    [GPUShaderStageRTX.RAY_INTERSECTION]: 'rint',
    [GPUShaderStageRTX.RAY_CLOSEST_HIT]: 'rchit',
    [GPUShaderStageRTX.RAY_ANY_HIT]: 'rahit',
  };
  let maxHitAttributesNumWords = MIN_HIT_ATTRIBUTES_WORDS;
  let functionDecls: string[] = [];
  let functionDefs: string[] = [];
  let functionInvocations: [number, string][] = [];
  // [global_index_in_stages]
  const stages: Map<_GPUShaderStageRTX, number[]> = new Map();
  for (let i = 0; i < descriptor.stages.length; i++) {
    const stg = descriptor.stages[i];
    let shaders_in_stage = stages.get(stg.stage);
    if (!shaders_in_stage) {
      shaders_in_stage = [];
      stages.set(stg.stage, shaders_in_stage);
    }
    // TODO: maybe call a common util function to get shader handle
    const indexWithinStage = shaders_in_stage.length; // this is the shader handle!
    const newName = canonicalShaderStageEntryName(stg.stage, indexWithinStage);
    console.debug('processing shader stage, global', i, 'local', indexWithinStage);
    const processed = glslTranspiler.process(stg.glslCode, SHADER_STAGE_NAME[stg.stage], stg.entryPoint, newName);
    maxUsedBindSet = Math.max(maxUsedBindSet, processed.max_bind_set_number);
    shaders_in_stage.push(i);
    maxHitAttributesNumWords = Math.max(maxHitAttributesNumWords, processed.hit_attributes_num_words);

    functionDecls.push(processed.forward_type_declarations() + processed.processed_entry_point_prototype() + ';');
    functionDefs.push(processed.processed_shader());
    functionInvocations.push([indexWithinStage, `{${processed.unpacking_code()} ${processed.invocation_code()} ${processed.packing_code()}}`]);
  }

  // TODO: impl
  const [bvhGeometriesDescs, bvhReferencedGeoBuffers] = (tlas as GPURayTracingAccelerationContainer_top_Impl).getBvhGeometryBuffersAndDescriptors();
  const numGeomBuffers = bvhReferencedGeoBuffers.size;
  const bvhGeometriesDescArray = bvhGeometriesDescs.map(
    d => `{${[
      d.vBufferIndex,
      d.iBufferIndex,
      d.vboOffset,
      d.vboStride,
      (d.vioOffset || 0),
      (d.vioStride || 0),
      d.owningGeometryType_todo_deprecate,
      d.owningGeometryFlags,
    ].join(',')}}`).join(',');
  const userPrelude = `
const uint ${GLOBAL_NAME__USER_NEXT_UNUSED_BIND_SET} = ${maxUsedBindSet + 1};
const uint ${GLOBAL_NAME__HIT_ATTRIBUTES_MAX_WORDS} = ${maxHitAttributesNumWords};
#define _CRT_USER_BVH_GEOM_BUFFERS_INITIALIZER_LIST {${bvhGeometriesDescArray}}
#define _CRT_USER_DEFINE_GEO_BUFFERS DEFINE_GEO_BUFFER_x${numGeomBuffers}
#define _CRT_USER_GEO_BUFFERS_ACCESSOR_CASES(wordIndex) _GET_FROM_BUFFER_CASE_x${numGeomBuffers}(wordIndex)
  `;

  const userFunctionsTable = Object.values(GPUShaderStageRTX).map(stg => {
    let shaders_in_stage = stages.get(stg) || [];
    if (shaders_in_stage.length === 1) {
      return `#define ${USER_FUNCTIONS_TABLE_NAME[stg]}  {${functionInvocations[shaders_in_stage[0]][1]}}`;
    }
    return `#define ${USER_FUNCTIONS_TABLE_NAME[stg]} \
    switch (${SHADER_STAGE_NAME[stg]}) { \
       ${shaders_in_stage.map(gi => { const f = functionInvocations[gi]; return `case ${f[0]}: {${f[1]}} break;`; }).join(' ')} \
    }`;
  }).join('\n') + '\n';

  // TODO: for functionDefs, analyze in => out dependency: dependency between ray_payload defs and references, 
  const completeGLSL = [glslRtPrelude, userPrelude, functionDecls.join('\n'), userFunctionsTable, glslRtEngineCode, functionDefs.join('\n')].join('\n');
  console.debug('completeGLSL', completeGLSL);
  // glsl -> spv -> wgsl
  const spirv = glslang.compileGLSL(completeGLSL, 'compute', false);
  const moduleIndex = naga.spv_in(new Uint8Array(spirv.buffer));
  const wgsl = naga.wgsl_out(moduleIndex);
  console.debug('completeWGSL:', wgsl);
  //! // TODO: only compose those shaders referenced in the groups
  const module = device.createShaderModule({ code: wgsl });
  // spirv.free();
  return [module, maxUsedBindSet + 1];
}
