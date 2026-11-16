module std.compiler;

public import std.agent.ToolRef;

// Opaque compiler metadata is descriptive data. The role controls its native
// value representation and keeps construction inside the compiler.

@std.role("std.compiler.ActionSummary")
public type ActionSummary;

@std.role("std.compiler.AgentId")
public type AgentId;

@std.role("std.compiler.CallableId")
public type CallableId;

@std.role("std.compiler.DefinitionId")
public type DefinitionId;

@std.role("std.compiler.DescriptorDetail")
public type DescriptorDetail;

@std.role("std.compiler.Determinism")
public type Determinism;

@std.role("std.compiler.EffectRowInfo")
public type EffectRowInfo;

@std.role("std.compiler.GenericOwnerInfo")
public type GenericOwnerInfo;

@std.role("std.compiler.StaticValueInfo")
public type StaticValueInfo;


@std.role("std.compiler.TraceConstraintInfo")
public type TraceConstraintInfo;

@std.role("std.compiler.TraceSummary")
public type TraceSummary;

@std.role("std.compiler.TraceWitness")
public type TraceWitness;

@std.role("std.compiler.TypeInfo")
public type TypeInfo;
