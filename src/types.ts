/// <reference types="@webgpu/types" />

export type _GPUShaderStageRTX = number & { readonly brand: unique symbol };
export type _GPURayTracingAccelerationContainerUsage = number & { readonly brand: unique symbol };
export type _GPURayTracingAccelerationGeometryUsage = number & { readonly brand: unique symbol };
export type _GPURayTracingAccelerationInstanceUsage = number & { readonly brand: unique symbol };

// Extends existing WebGPU interfaces
declare global {
  interface GPUDevice {
    /**
     * The size in bytes of the shader header.
     */
    readonly ShaderGroupHandleSize: number;
    /**
     * The required alignment in bytes for the base of the shader binding table.
     */
    readonly ShaderGroupBaseAlignment: number;
    /**
     * The required alignment in bytes for each shader binding table entry.
     */
    readonly ShaderGroupHandleAlignment: number;
    /**
     * The maximum allowed size for the shader record stride.
     */
    readonly ShaderGroupRecordMaxStride: number;
    /**
     * Creates a new acceleration structure object. Current implementation does
     * not support creating bottom-level acceleration structures separately,
     * they must be part of the top-level acceleration structure description.
     * @param descriptor - Description of the {@link GPURayTracingAccelerationContainer_top} to create.
     */
    createRayTracingAccelerationContainer(descriptor: GPURayTracingAccelerationContainerDescriptor_top): GPURayTracingAccelerationContainer_top;
    /**
     * Builds an acceleration structure on the host.
     * @param container - The {@link GPURayTracingAccelerationContainer_top} to be built.
     */
    hostBuildRayTracingAccelerationContainer(container: GPURayTracingAccelerationContainer_top): void;
    /**
     * Creates a new ray tracing pipeline object.
     * @param descriptor - Description of the {@link GPURayTracingPipeline} to create.
     * @param tlas - The built {@link GPURayTracingAccelerationContainer_top}.
     *              Current implementation requires it to properly set states
     *              in the combined shader of the ray tracing pipeline.
     */
    createRayTracingPipeline(descriptor: GPURayTracingPipelineDescriptor, tlas: GPURayTracingAccelerationContainer_top): Promise<GPURayTracingPipeline>;
  }

  interface GPUCommandEncoder {
    /**
     * Begins encoding a ray tracing pass.
     */
    beginRayTracingPass(): GPURayTracingPassEncoder;
  }
}

// Global variables
declare global {
  /**
   * A special handle denoting that all shaders in the hit group are not used.
   */
  var WEBRTX_HIT_GROUP_ALL_SHADERS_UNUSED_HANDLE: number;

  // TODO: figure out how to extend GPUBufferUsage and GPUShaderStage instead of
  // introducing types and variables
  var GPUBufferUsageRTX: {
    /**
     * VK_BUFFER_USAGE_ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_BIT_KHR 
     */
    ACCELERATION_STRUCTURE_BUILD_INPUT_READONLY: GPUFlagsConstant,
    /**
     * VK_BUFFER_USAGE_SHADER_BINDING_TABLE_BIT_KHR
     */
    SHADER_BINDING_TABLE: GPUFlagsConstant,
  };

  /**
   * Ray tracing extension specific shader stages.
   */
  var GPUShaderStageRTX: {
    /**
     * Ray generation shader.
     */
    RAY_GENERATION: _GPUShaderStageRTX,
    /**
     * Ray any-hit shader, called whenever a ray hit occurs.
     */
    RAY_ANY_HIT: _GPUShaderStageRTX,
    /**
     * Ray closest-hit shader, called once for the closest hit along a ray.
     */
    RAY_CLOSEST_HIT: _GPUShaderStageRTX,
    /**
     * Ray miss shader, called when a ray didn't hit anything.
     */
    RAY_MISS: _GPUShaderStageRTX,
    /**
     * Ray intersection shader, implements ray-primitive intersections e.g. for
     * procedural geometries.
     */
    RAY_INTERSECTION: _GPUShaderStageRTX,
  };

  /**
   * Usage flags for GPURayTracingAccelerationContainer.
   */
  var GPURayTracingAccelerationContainerUsage: {
    NONE: _GPURayTracingAccelerationContainerUsage,
    // TODO: implement these
    // ALLOW_UPDATE: _GPURayTracingAccelerationContainerUsage,
    // ALLOW_COMPACTION: _GPURayTracingAccelerationContainerUsage,
    // PREFER_FAST_TRACE: _GPURayTracingAccelerationContainerUsage,
    // PREFER_FAST_BUILD: _GPURayTracingAccelerationContainerUsage,
    // LOW_MEMORY: _GPURayTracingAccelerationContainerUsage,
  };

  /**
   * Usage flags for geometries in GPURayTracingAccelerationContainer.
   */
  var GPURayTracingAccelerationGeometryUsage: {
    NONE: _GPURayTracingAccelerationGeometryUsage,
    // TODO: implement these
    // OPAQUE: _GPURayTracingAccelerationGeometryUsage,
    // NO_DUPLICATE_ANY_HIT_INVOCATION: _GPURayTracingAccelerationGeometryUsage,
  };

  /**
   * Usage flags for instances in GPURayTracingAccelerationContainer.
   */
  var GPURayTracingAccelerationInstanceUsage: {
    NONE: _GPURayTracingAccelerationInstanceUsage,
    // TODO: implement these
    // TRIANGLE_FACING_CULL_DISABLE: _GPURayTracingAccelerationInstanceUsage,
    // TRIANGLE_FRONT_COUNTERCLOCKWISE: _GPURayTracingAccelerationInstanceUsage,
    // FORCE_OPAQUE: _GPURayTracingAccelerationInstanceUsage,
    // FORCE_NO_OPAQUE: _GPURayTracingAccelerationInstanceUsage,
  };
}

// Global types
declare global {
  type GPURayTracingAccelerationContainerLevel =
    | 'bottom'
    | 'top';

  interface GPURayTracingAccelerationGeometryVertexDescriptor
    extends GPUBufferBinding {
    format: 'float32x3', // TODO: support GPUVertexFormat;
    stride: GPUSize64;
  }

  interface GPURayTracingAccelerationGeometryIndexDescriptor
    extends GPUBufferBinding {
    format: 'uint32', // TODO: support GPUIndexFormat;
  }

  interface GPURayTracingAccelerationGeometryAABBDescriptor
    extends GPUBufferBinding {
    format: 'float32x2';
    stride: GPUSize64;
  }

  interface GPURayTracingAccelerationGeometryDescriptor_triangles {
    usage: _GPURayTracingAccelerationGeometryUsage;
    type: 'triangles';
    vertex: GPURayTracingAccelerationGeometryVertexDescriptor;
    index?: GPURayTracingAccelerationGeometryIndexDescriptor;
    // TODO: support optional transform matrix
  }

  interface GPURayTracingAccelerationGeometryDescriptor_aabbs {
    usage: _GPURayTracingAccelerationGeometryUsage;
    type: 'aabbs';
    aabb: GPURayTracingAccelerationGeometryAABBDescriptor;
  }

  type GPURayTracingAccelerationGeometryDescriptor =
    | GPURayTracingAccelerationGeometryDescriptor_triangles
    | GPURayTracingAccelerationGeometryDescriptor_aabbs;

  interface GPURayTracingAccelerationInstanceDescriptor {
    usage: _GPURayTracingAccelerationInstanceUsage;
    /**
     * An 8-bit visibility mask for the geometry. Currently not used.
     */
    mask: number;
    /**
     * A 24-bit user-specified index value accessible to ray shaders via gl_InstanceCustomIndex.
     */
    instanceCustomIndex?: number;
    /**
     * A 24-bit offset used in calculating the hit shader binding table index.
     */
    instanceSBTRecordOffset: number;
    /**
     * 3x4 row-major affine transform matrix.
     */
    transformMatrix?: Float32Array;
    // TODO: instead of specifying not-built descriptor, allow built TLAS or BLAS
    blas: GPURayTracingAccelerationContainerDescriptor_bottom,
  }

  interface GPURayTracingAccelerationContainerDescriptor_bottom {
    usage: _GPURayTracingAccelerationContainerUsage;
    level: 'bottom';
    // TODO: geometries should be of only single type, not both.
    geometries: GPURayTracingAccelerationGeometryDescriptor[];
  }

  interface GPURayTracingAccelerationContainerDescriptor_top {
    usage: _GPURayTracingAccelerationContainerUsage;
    level: 'top';
    instances: GPURayTracingAccelerationInstanceDescriptor[];
  }

  interface GPURayTracingShaderStageDescriptor {
    stage: _GPUShaderStageRTX;
    /**
     * Code for the shader stage, currently only GLSL_EXT_ray_tracing is supported.
     */
    glslCode: string;
    entryPoint: string;
  }

  interface GPURayTracingShaderTrianglesHitGroupDescriptor {
    type: 'triangles-hit-group';
    closestHitIndex?: number;
    anyHitIndex?: number;
  }

  interface GPURayTracingShaderProceduralHitGroupDescriptor {
    type: 'procedural-hit-group';
    intersectionIndex: number;
    closestHitIndex?: number;
    anyHitIndex?: number;
  }

  interface GPURayTracingShaderGeneralGroupDescriptor {
    type: 'general';
    generalIndex: number;
  }

  type GPURayTracingShaderGroupDescriptor =
    | GPURayTracingShaderGeneralGroupDescriptor
    | GPURayTracingShaderTrianglesHitGroupDescriptor
    | GPURayTracingShaderProceduralHitGroupDescriptor;

  // format
  // 00 rahit rchit int
  type ShaderGroupHandle = number;

  interface BufferRegion {
    // TODO: allow using different buffers per shader, see GPUShaderBindingTable
    // buffer: GPUBuffer; 
    start: GPUSize32;
    stride: GPUSize32;
    size: GPUSize32;
  }

  /**
   * Shader binding table consists of a set of shader function handles and
   * embedded parameters for these functions. 
   */
  interface GPUShaderBindingTable {
    buffer: GPUBuffer;
    rayGen: BufferRegion,
    rayMiss: BufferRegion,
    rayHit: BufferRegion,
    callable: BufferRegion,
  }

  interface GPURayTracingPipelineDescriptor {
    // TODO: allow specifying layout
    /**
     * The set of the shader stages to be included in the ray tracing pipeline.
     */
    stages: GPURayTracingShaderStageDescriptor[];
    /**
     * The set of the shader stages to be included in each shader group in the ray tracing pipeline.
     */
    groups: GPURayTracingShaderGroupDescriptor[];
  }

  interface GPURayTracingPipeline {
    getBindGroupLayout(index: number): GPUBindGroupLayout;
    /**
     * Query ray tracing pipeline shader group handles, see vkGetRayTracingShaderGroupHandlesKHR.
     * @param first 
     * @param count 
     */
    getShaderGroupHandles(first: number, count: number): ShaderGroupHandle[];
  }

  interface GPURayTracingPassEncoder {
    setPipeline(pipeline: GPURayTracingPipeline): void;
    setBindGroup(index: GPUIndex32, bindGroup: GPUBindGroup): void;
    /**
     * Initializes a ray tracing dispatch.
     * @param device 
     * @param sbt - The shader binding table data to be used in this pass.
     * @param width - The width of the ray trace query dimensions.
     * @param height - The height of the ray trace query dimensions.
     * @param depth - The depth of the ray trace query dimensions.
     */
    traceRays(
      device: GPUDevice,
      sbt: GPUShaderBindingTable,
      width: GPUSize32,
      height: GPUSize32,
      depth?: GPUSize32,
    ): void;
    end(): void;
  }

  // TODO: extends GPUBindingResource
  interface GPURayTracingAccelerationContainer_top {
  }
}
