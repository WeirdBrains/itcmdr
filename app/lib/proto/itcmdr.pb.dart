// This is a generated file - do not edit.
//
// Generated from itcmdr.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports

import 'dart:core' as $core;

import 'package:fixnum/fixnum.dart' as $fixnum;
import 'package:protobuf/protobuf.dart' as $pb;

import 'itcmdr.pbenum.dart';

export 'package:protobuf/protobuf.dart' show GeneratedMessageGenericExtensions;

export 'itcmdr.pbenum.dart';

/// Machine summary for overview lists.
class Machine extends $pb.GeneratedMessage {
  factory Machine({
    $core.String? name,
    $core.String? status,
    $core.int? openIncidents,
    $fixnum.Int64? lastRunTs,
  }) {
    final result = Machine._();
    if (name != null) result.name = name;
    if (status != null) result.status = status;
    if (openIncidents != null) result.openIncidents = openIncidents;
    if (lastRunTs != null) result.lastRunTs = lastRunTs;
    return result;
  }

  Machine._();

  factory Machine.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      Machine()..mergeFromBuffer(data, registry);
  factory Machine.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      Machine()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'Machine',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: Machine.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'name')
    ..aOS(2, _omitFieldNames ? '' : 'status')
    ..aI(3, _omitFieldNames ? '' : 'openIncidents')
    ..aInt64(4, _omitFieldNames ? '' : 'lastRunTs')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Machine clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Machine copyWith(void Function(Machine) updates) =>
      super.copyWith((message) => updates(message as Machine)) as Machine;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use Machine() / Machine.new instead')
  static Machine create() => Machine._();
  static $pb.GeneratedMessage $_createMessage() => Machine._();
  @$core.override
  Machine createEmptyInstance() => Machine._();
  @$core.pragma('dart2js:noInline')
  static Machine getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<Machine>(Machine.$_createMessage);
  static Machine? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get name => $_getSZ(0);
  @$pb.TagNumber(1)
  set name($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasName() => $_has(0);
  @$pb.TagNumber(1)
  void clearName() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get status => $_getSZ(1);
  @$pb.TagNumber(2)
  set status($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasStatus() => $_has(1);
  @$pb.TagNumber(2)
  void clearStatus() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.int get openIncidents => $_getIZ(2);
  @$pb.TagNumber(3)
  set openIncidents($core.int value) => $_setSignedInt32(2, value);
  @$pb.TagNumber(3)
  $core.bool hasOpenIncidents() => $_has(2);
  @$pb.TagNumber(3)
  void clearOpenIncidents() => $_clearField(3);

  @$pb.TagNumber(4)
  $fixnum.Int64 get lastRunTs => $_getI64(3);
  @$pb.TagNumber(4)
  set lastRunTs($fixnum.Int64 value) => $_setInt64(3, value);
  @$pb.TagNumber(4)
  $core.bool hasLastRunTs() => $_has(3);
  @$pb.TagNumber(4)
  void clearLastRunTs() => $_clearField(4);
}

class ListMachinesRequest extends $pb.GeneratedMessage {
  factory ListMachinesRequest() => ListMachinesRequest._();

  ListMachinesRequest._();

  factory ListMachinesRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListMachinesRequest()..mergeFromBuffer(data, registry);
  factory ListMachinesRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListMachinesRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListMachinesRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListMachinesRequest.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListMachinesRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListMachinesRequest copyWith(void Function(ListMachinesRequest) updates) =>
      super.copyWith((message) => updates(message as ListMachinesRequest))
          as ListMachinesRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core
      .Deprecated('Use ListMachinesRequest() / ListMachinesRequest.new instead')
  static ListMachinesRequest create() => ListMachinesRequest._();
  static $pb.GeneratedMessage $_createMessage() => ListMachinesRequest._();
  @$core.override
  ListMachinesRequest createEmptyInstance() => ListMachinesRequest._();
  @$core.pragma('dart2js:noInline')
  static ListMachinesRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListMachinesRequest>(
          ListMachinesRequest.$_createMessage);
  static ListMachinesRequest? _defaultInstance;
}

class ListMachinesResponse extends $pb.GeneratedMessage {
  factory ListMachinesResponse({
    $core.Iterable<Machine>? machines,
  }) {
    final result = ListMachinesResponse._();
    if (machines != null) result.machines.addAll(machines);
    return result;
  }

  ListMachinesResponse._();

  factory ListMachinesResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListMachinesResponse()..mergeFromBuffer(data, registry);
  factory ListMachinesResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListMachinesResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListMachinesResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListMachinesResponse.$_createMessage)
    ..pPM<Machine>(1, _omitFieldNames ? '' : 'machines',
        subBuilder: Machine.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListMachinesResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListMachinesResponse copyWith(void Function(ListMachinesResponse) updates) =>
      super.copyWith((message) => updates(message as ListMachinesResponse))
          as ListMachinesResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use ListMachinesResponse() / ListMachinesResponse.new instead')
  static ListMachinesResponse create() => ListMachinesResponse._();
  static $pb.GeneratedMessage $_createMessage() => ListMachinesResponse._();
  @$core.override
  ListMachinesResponse createEmptyInstance() => ListMachinesResponse._();
  @$core.pragma('dart2js:noInline')
  static ListMachinesResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListMachinesResponse>(
          ListMachinesResponse.$_createMessage);
  static ListMachinesResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<Machine> get machines => $_getList(0);
}

/// Request to trigger runs on one or more machines.
class TriggerRunsRequest extends $pb.GeneratedMessage {
  factory TriggerRunsRequest({
    $core.Iterable<$core.String>? machines,
    $core.bool? reportOnly,
    $core.bool? repair,
    $core.bool? force,
    $core.Iterable<$core.String>? categories,
    $core.Iterable<$core.String>? plugins,
  }) {
    final result = TriggerRunsRequest._();
    if (machines != null) result.machines.addAll(machines);
    if (reportOnly != null) result.reportOnly = reportOnly;
    if (repair != null) result.repair = repair;
    if (force != null) result.force = force;
    if (categories != null) result.categories.addAll(categories);
    if (plugins != null) result.plugins.addAll(plugins);
    return result;
  }

  TriggerRunsRequest._();

  factory TriggerRunsRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      TriggerRunsRequest()..mergeFromBuffer(data, registry);
  factory TriggerRunsRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      TriggerRunsRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'TriggerRunsRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: TriggerRunsRequest.$_createMessage)
    ..pPS(1, _omitFieldNames ? '' : 'machines')
    ..aOB(2, _omitFieldNames ? '' : 'reportOnly')
    ..aOB(3, _omitFieldNames ? '' : 'repair')
    ..aOB(4, _omitFieldNames ? '' : 'force')
    ..pPS(5, _omitFieldNames ? '' : 'categories')
    ..pPS(6, _omitFieldNames ? '' : 'plugins')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  TriggerRunsRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  TriggerRunsRequest copyWith(void Function(TriggerRunsRequest) updates) =>
      super.copyWith((message) => updates(message as TriggerRunsRequest))
          as TriggerRunsRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use TriggerRunsRequest() / TriggerRunsRequest.new instead')
  static TriggerRunsRequest create() => TriggerRunsRequest._();
  static $pb.GeneratedMessage $_createMessage() => TriggerRunsRequest._();
  @$core.override
  TriggerRunsRequest createEmptyInstance() => TriggerRunsRequest._();
  @$core.pragma('dart2js:noInline')
  static TriggerRunsRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<TriggerRunsRequest>(
          TriggerRunsRequest.$_createMessage);
  static TriggerRunsRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<$core.String> get machines => $_getList(0);

  @$pb.TagNumber(2)
  $core.bool get reportOnly => $_getBF(1);
  @$pb.TagNumber(2)
  set reportOnly($core.bool value) => $_setBool(1, value);
  @$pb.TagNumber(2)
  $core.bool hasReportOnly() => $_has(1);
  @$pb.TagNumber(2)
  void clearReportOnly() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.bool get repair => $_getBF(2);
  @$pb.TagNumber(3)
  set repair($core.bool value) => $_setBool(2, value);
  @$pb.TagNumber(3)
  $core.bool hasRepair() => $_has(2);
  @$pb.TagNumber(3)
  void clearRepair() => $_clearField(3);

  @$pb.TagNumber(4)
  $core.bool get force => $_getBF(3);
  @$pb.TagNumber(4)
  set force($core.bool value) => $_setBool(3, value);
  @$pb.TagNumber(4)
  $core.bool hasForce() => $_has(3);
  @$pb.TagNumber(4)
  void clearForce() => $_clearField(4);

  @$pb.TagNumber(5)
  $pb.PbList<$core.String> get categories => $_getList(4);

  @$pb.TagNumber(6)
  $pb.PbList<$core.String> get plugins => $_getList(5);
}

class TriggerRunsResponse extends $pb.GeneratedMessage {
  factory TriggerRunsResponse({
    $core.String? requestId,
  }) {
    final result = TriggerRunsResponse._();
    if (requestId != null) result.requestId = requestId;
    return result;
  }

  TriggerRunsResponse._();

  factory TriggerRunsResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      TriggerRunsResponse()..mergeFromBuffer(data, registry);
  factory TriggerRunsResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      TriggerRunsResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'TriggerRunsResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: TriggerRunsResponse.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'requestId')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  TriggerRunsResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  TriggerRunsResponse copyWith(void Function(TriggerRunsResponse) updates) =>
      super.copyWith((message) => updates(message as TriggerRunsResponse))
          as TriggerRunsResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core
      .Deprecated('Use TriggerRunsResponse() / TriggerRunsResponse.new instead')
  static TriggerRunsResponse create() => TriggerRunsResponse._();
  static $pb.GeneratedMessage $_createMessage() => TriggerRunsResponse._();
  @$core.override
  TriggerRunsResponse createEmptyInstance() => TriggerRunsResponse._();
  @$core.pragma('dart2js:noInline')
  static TriggerRunsResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<TriggerRunsResponse>(
          TriggerRunsResponse.$_createMessage);
  static TriggerRunsResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get requestId => $_getSZ(0);
  @$pb.TagNumber(1)
  set requestId($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasRequestId() => $_has(0);
  @$pb.TagNumber(1)
  void clearRequestId() => $_clearField(1);
}

class GetRequestStatusRequest extends $pb.GeneratedMessage {
  factory GetRequestStatusRequest({
    $core.String? requestId,
  }) {
    final result = GetRequestStatusRequest._();
    if (requestId != null) result.requestId = requestId;
    return result;
  }

  GetRequestStatusRequest._();

  factory GetRequestStatusRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetRequestStatusRequest()..mergeFromBuffer(data, registry);
  factory GetRequestStatusRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetRequestStatusRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetRequestStatusRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: GetRequestStatusRequest.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'requestId')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetRequestStatusRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetRequestStatusRequest copyWith(
          void Function(GetRequestStatusRequest) updates) =>
      super.copyWith((message) => updates(message as GetRequestStatusRequest))
          as GetRequestStatusRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use GetRequestStatusRequest() / GetRequestStatusRequest.new instead')
  static GetRequestStatusRequest create() => GetRequestStatusRequest._();
  static $pb.GeneratedMessage $_createMessage() => GetRequestStatusRequest._();
  @$core.override
  GetRequestStatusRequest createEmptyInstance() => GetRequestStatusRequest._();
  @$core.pragma('dart2js:noInline')
  static GetRequestStatusRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetRequestStatusRequest>(
          GetRequestStatusRequest.$_createMessage);
  static GetRequestStatusRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get requestId => $_getSZ(0);
  @$pb.TagNumber(1)
  set requestId($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasRequestId() => $_has(0);
  @$pb.TagNumber(1)
  void clearRequestId() => $_clearField(1);
}

class RequestStatus extends $pb.GeneratedMessage {
  factory RequestStatus({
    RequestStatus_Status? status,
    $core.String? message,
  }) {
    final result = RequestStatus._();
    if (status != null) result.status = status;
    if (message != null) result.message = message;
    return result;
  }

  RequestStatus._();

  factory RequestStatus.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      RequestStatus()..mergeFromBuffer(data, registry);
  factory RequestStatus.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      RequestStatus()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'RequestStatus',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: RequestStatus.$_createMessage)
    ..aE<RequestStatus_Status>(1, _omitFieldNames ? '' : 'status',
        enumValues: RequestStatus_Status.values)
    ..aOS(2, _omitFieldNames ? '' : 'message')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  RequestStatus clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  RequestStatus copyWith(void Function(RequestStatus) updates) =>
      super.copyWith((message) => updates(message as RequestStatus))
          as RequestStatus;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use RequestStatus() / RequestStatus.new instead')
  static RequestStatus create() => RequestStatus._();
  static $pb.GeneratedMessage $_createMessage() => RequestStatus._();
  @$core.override
  RequestStatus createEmptyInstance() => RequestStatus._();
  @$core.pragma('dart2js:noInline')
  static RequestStatus getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<RequestStatus>(
          RequestStatus.$_createMessage);
  static RequestStatus? _defaultInstance;

  @$pb.TagNumber(1)
  RequestStatus_Status get status => $_getN(0);
  @$pb.TagNumber(1)
  set status(RequestStatus_Status value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasStatus() => $_has(0);
  @$pb.TagNumber(1)
  void clearStatus() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get message => $_getSZ(1);
  @$pb.TagNumber(2)
  set message($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasMessage() => $_has(1);
  @$pb.TagNumber(2)
  void clearMessage() => $_clearField(2);
}

class GetRequestStatusResponse extends $pb.GeneratedMessage {
  factory GetRequestStatusResponse({
    RequestStatus_Status? status,
    $core.String? message,
  }) {
    final result = GetRequestStatusResponse._();
    if (status != null) result.status = status;
    if (message != null) result.message = message;
    return result;
  }

  GetRequestStatusResponse._();

  factory GetRequestStatusResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetRequestStatusResponse()..mergeFromBuffer(data, registry);
  factory GetRequestStatusResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetRequestStatusResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetRequestStatusResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: GetRequestStatusResponse.$_createMessage)
    ..aE<RequestStatus_Status>(1, _omitFieldNames ? '' : 'status',
        enumValues: RequestStatus_Status.values)
    ..aOS(2, _omitFieldNames ? '' : 'message')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetRequestStatusResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetRequestStatusResponse copyWith(
          void Function(GetRequestStatusResponse) updates) =>
      super.copyWith((message) => updates(message as GetRequestStatusResponse))
          as GetRequestStatusResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use GetRequestStatusResponse() / GetRequestStatusResponse.new instead')
  static GetRequestStatusResponse create() => GetRequestStatusResponse._();
  static $pb.GeneratedMessage $_createMessage() => GetRequestStatusResponse._();
  @$core.override
  GetRequestStatusResponse createEmptyInstance() =>
      GetRequestStatusResponse._();
  @$core.pragma('dart2js:noInline')
  static GetRequestStatusResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetRequestStatusResponse>(
          GetRequestStatusResponse.$_createMessage);
  static GetRequestStatusResponse? _defaultInstance;

  @$pb.TagNumber(1)
  RequestStatus_Status get status => $_getN(0);
  @$pb.TagNumber(1)
  set status(RequestStatus_Status value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasStatus() => $_has(0);
  @$pb.TagNumber(1)
  void clearStatus() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get message => $_getSZ(1);
  @$pb.TagNumber(2)
  set message($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasMessage() => $_has(1);
  @$pb.TagNumber(2)
  void clearMessage() => $_clearField(2);
}

class GetMetricsRequest extends $pb.GeneratedMessage {
  factory GetMetricsRequest({
    $core.int? windowDays,
    Rollup? rollup,
  }) {
    final result = GetMetricsRequest._();
    if (windowDays != null) result.windowDays = windowDays;
    if (rollup != null) result.rollup = rollup;
    return result;
  }

  GetMetricsRequest._();

  factory GetMetricsRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetMetricsRequest()..mergeFromBuffer(data, registry);
  factory GetMetricsRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetMetricsRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetMetricsRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: GetMetricsRequest.$_createMessage)
    ..aI(1, _omitFieldNames ? '' : 'windowDays')
    ..aE<Rollup>(2, _omitFieldNames ? '' : 'rollup', enumValues: Rollup.values)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetMetricsRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetMetricsRequest copyWith(void Function(GetMetricsRequest) updates) =>
      super.copyWith((message) => updates(message as GetMetricsRequest))
          as GetMetricsRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use GetMetricsRequest() / GetMetricsRequest.new instead')
  static GetMetricsRequest create() => GetMetricsRequest._();
  static $pb.GeneratedMessage $_createMessage() => GetMetricsRequest._();
  @$core.override
  GetMetricsRequest createEmptyInstance() => GetMetricsRequest._();
  @$core.pragma('dart2js:noInline')
  static GetMetricsRequest getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<GetMetricsRequest>(
          GetMetricsRequest.$_createMessage);
  static GetMetricsRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.int get windowDays => $_getIZ(0);
  @$pb.TagNumber(1)
  set windowDays($core.int value) => $_setSignedInt32(0, value);
  @$pb.TagNumber(1)
  $core.bool hasWindowDays() => $_has(0);
  @$pb.TagNumber(1)
  void clearWindowDays() => $_clearField(1);

  @$pb.TagNumber(2)
  Rollup get rollup => $_getN(1);
  @$pb.TagNumber(2)
  set rollup(Rollup value) => $_setField(2, value);
  @$pb.TagNumber(2)
  $core.bool hasRollup() => $_has(1);
  @$pb.TagNumber(2)
  void clearRollup() => $_clearField(2);
}

class MetricPoint extends $pb.GeneratedMessage {
  factory MetricPoint({
    $core.String? key,
    $fixnum.Int64? ts,
    $core.double? value,
  }) {
    final result = MetricPoint._();
    if (key != null) result.key = key;
    if (ts != null) result.ts = ts;
    if (value != null) result.value = value;
    return result;
  }

  MetricPoint._();

  factory MetricPoint.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      MetricPoint()..mergeFromBuffer(data, registry);
  factory MetricPoint.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      MetricPoint()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'MetricPoint',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: MetricPoint.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'key')
    ..aInt64(2, _omitFieldNames ? '' : 'ts')
    ..aD(3, _omitFieldNames ? '' : 'value')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  MetricPoint clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  MetricPoint copyWith(void Function(MetricPoint) updates) =>
      super.copyWith((message) => updates(message as MetricPoint))
          as MetricPoint;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use MetricPoint() / MetricPoint.new instead')
  static MetricPoint create() => MetricPoint._();
  static $pb.GeneratedMessage $_createMessage() => MetricPoint._();
  @$core.override
  MetricPoint createEmptyInstance() => MetricPoint._();
  @$core.pragma('dart2js:noInline')
  static MetricPoint getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<MetricPoint>(
          MetricPoint.$_createMessage);
  static MetricPoint? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get key => $_getSZ(0);
  @$pb.TagNumber(1)
  set key($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasKey() => $_has(0);
  @$pb.TagNumber(1)
  void clearKey() => $_clearField(1);

  @$pb.TagNumber(2)
  $fixnum.Int64 get ts => $_getI64(1);
  @$pb.TagNumber(2)
  set ts($fixnum.Int64 value) => $_setInt64(1, value);
  @$pb.TagNumber(2)
  $core.bool hasTs() => $_has(1);
  @$pb.TagNumber(2)
  void clearTs() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.double get value => $_getN(2);
  @$pb.TagNumber(3)
  set value($core.double value) => $_setDouble(2, value);
  @$pb.TagNumber(3)
  $core.bool hasValue() => $_has(2);
  @$pb.TagNumber(3)
  void clearValue() => $_clearField(3);
}

class GetMetricsResponse extends $pb.GeneratedMessage {
  factory GetMetricsResponse({
    $core.Iterable<MetricPoint>? points,
  }) {
    final result = GetMetricsResponse._();
    if (points != null) result.points.addAll(points);
    return result;
  }

  GetMetricsResponse._();

  factory GetMetricsResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetMetricsResponse()..mergeFromBuffer(data, registry);
  factory GetMetricsResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetMetricsResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetMetricsResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: GetMetricsResponse.$_createMessage)
    ..pPM<MetricPoint>(1, _omitFieldNames ? '' : 'points',
        subBuilder: MetricPoint.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetMetricsResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetMetricsResponse copyWith(void Function(GetMetricsResponse) updates) =>
      super.copyWith((message) => updates(message as GetMetricsResponse))
          as GetMetricsResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use GetMetricsResponse() / GetMetricsResponse.new instead')
  static GetMetricsResponse create() => GetMetricsResponse._();
  static $pb.GeneratedMessage $_createMessage() => GetMetricsResponse._();
  @$core.override
  GetMetricsResponse createEmptyInstance() => GetMetricsResponse._();
  @$core.pragma('dart2js:noInline')
  static GetMetricsResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetMetricsResponse>(
          GetMetricsResponse.$_createMessage);
  static GetMetricsResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<MetricPoint> get points => $_getList(0);
}

/// Exceptions allowlist rules
class ExceptionScope extends $pb.GeneratedMessage {
  factory ExceptionScope({
    $core.Iterable<$core.String>? machines,
    $core.String? role,
  }) {
    final result = ExceptionScope._();
    if (machines != null) result.machines.addAll(machines);
    if (role != null) result.role = role;
    return result;
  }

  ExceptionScope._();

  factory ExceptionScope.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ExceptionScope()..mergeFromBuffer(data, registry);
  factory ExceptionScope.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ExceptionScope()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ExceptionScope',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ExceptionScope.$_createMessage)
    ..pPS(1, _omitFieldNames ? '' : 'machines')
    ..aOS(2, _omitFieldNames ? '' : 'role')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ExceptionScope clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ExceptionScope copyWith(void Function(ExceptionScope) updates) =>
      super.copyWith((message) => updates(message as ExceptionScope))
          as ExceptionScope;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use ExceptionScope() / ExceptionScope.new instead')
  static ExceptionScope create() => ExceptionScope._();
  static $pb.GeneratedMessage $_createMessage() => ExceptionScope._();
  @$core.override
  ExceptionScope createEmptyInstance() => ExceptionScope._();
  @$core.pragma('dart2js:noInline')
  static ExceptionScope getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<ExceptionScope>(
          ExceptionScope.$_createMessage);
  static ExceptionScope? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<$core.String> get machines => $_getList(0);

  @$pb.TagNumber(2)
  $core.String get role => $_getSZ(1);
  @$pb.TagNumber(2)
  set role($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasRole() => $_has(1);
  @$pb.TagNumber(2)
  void clearRole() => $_clearField(2);
}

class ExceptionRule extends $pb.GeneratedMessage {
  factory ExceptionRule({
    $core.String? id,
    ExceptionScope? scope,
    $core.String? checkId,
    $core.String? checkPattern,
    $core.String? action,
    $core.String? reason,
    $core.String? expires,
    $core.bool? active,
  }) {
    final result = ExceptionRule._();
    if (id != null) result.id = id;
    if (scope != null) result.scope = scope;
    if (checkId != null) result.checkId = checkId;
    if (checkPattern != null) result.checkPattern = checkPattern;
    if (action != null) result.action = action;
    if (reason != null) result.reason = reason;
    if (expires != null) result.expires = expires;
    if (active != null) result.active = active;
    return result;
  }

  ExceptionRule._();

  factory ExceptionRule.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ExceptionRule()..mergeFromBuffer(data, registry);
  factory ExceptionRule.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ExceptionRule()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ExceptionRule',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ExceptionRule.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'id')
    ..aOM<ExceptionScope>(2, _omitFieldNames ? '' : 'scope',
        subBuilder: ExceptionScope.$_createMessage)
    ..aOS(3, _omitFieldNames ? '' : 'checkId')
    ..aOS(4, _omitFieldNames ? '' : 'checkPattern')
    ..aOS(5, _omitFieldNames ? '' : 'action')
    ..aOS(6, _omitFieldNames ? '' : 'reason')
    ..aOS(7, _omitFieldNames ? '' : 'expires')
    ..aOB(8, _omitFieldNames ? '' : 'active')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ExceptionRule clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ExceptionRule copyWith(void Function(ExceptionRule) updates) =>
      super.copyWith((message) => updates(message as ExceptionRule))
          as ExceptionRule;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use ExceptionRule() / ExceptionRule.new instead')
  static ExceptionRule create() => ExceptionRule._();
  static $pb.GeneratedMessage $_createMessage() => ExceptionRule._();
  @$core.override
  ExceptionRule createEmptyInstance() => ExceptionRule._();
  @$core.pragma('dart2js:noInline')
  static ExceptionRule getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<ExceptionRule>(
          ExceptionRule.$_createMessage);
  static ExceptionRule? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get id => $_getSZ(0);
  @$pb.TagNumber(1)
  set id($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasId() => $_has(0);
  @$pb.TagNumber(1)
  void clearId() => $_clearField(1);

  @$pb.TagNumber(2)
  ExceptionScope get scope => $_getN(1);
  @$pb.TagNumber(2)
  set scope(ExceptionScope value) => $_setField(2, value);
  @$pb.TagNumber(2)
  $core.bool hasScope() => $_has(1);
  @$pb.TagNumber(2)
  void clearScope() => $_clearField(2);
  @$pb.TagNumber(2)
  ExceptionScope ensureScope() => $_ensure(1);

  @$pb.TagNumber(3)
  $core.String get checkId => $_getSZ(2);
  @$pb.TagNumber(3)
  set checkId($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasCheckId() => $_has(2);
  @$pb.TagNumber(3)
  void clearCheckId() => $_clearField(3);

  @$pb.TagNumber(4)
  $core.String get checkPattern => $_getSZ(3);
  @$pb.TagNumber(4)
  set checkPattern($core.String value) => $_setString(3, value);
  @$pb.TagNumber(4)
  $core.bool hasCheckPattern() => $_has(3);
  @$pb.TagNumber(4)
  void clearCheckPattern() => $_clearField(4);

  @$pb.TagNumber(5)
  $core.String get action => $_getSZ(4);
  @$pb.TagNumber(5)
  set action($core.String value) => $_setString(4, value);
  @$pb.TagNumber(5)
  $core.bool hasAction() => $_has(4);
  @$pb.TagNumber(5)
  void clearAction() => $_clearField(5);

  @$pb.TagNumber(6)
  $core.String get reason => $_getSZ(5);
  @$pb.TagNumber(6)
  set reason($core.String value) => $_setString(5, value);
  @$pb.TagNumber(6)
  $core.bool hasReason() => $_has(5);
  @$pb.TagNumber(6)
  void clearReason() => $_clearField(6);

  @$pb.TagNumber(7)
  $core.String get expires => $_getSZ(6);
  @$pb.TagNumber(7)
  set expires($core.String value) => $_setString(6, value);
  @$pb.TagNumber(7)
  $core.bool hasExpires() => $_has(6);
  @$pb.TagNumber(7)
  void clearExpires() => $_clearField(7);

  @$pb.TagNumber(8)
  $core.bool get active => $_getBF(7);
  @$pb.TagNumber(8)
  set active($core.bool value) => $_setBool(7, value);
  @$pb.TagNumber(8)
  $core.bool hasActive() => $_has(7);
  @$pb.TagNumber(8)
  void clearActive() => $_clearField(8);
}

class ListExceptionsRequest extends $pb.GeneratedMessage {
  factory ListExceptionsRequest({
    $core.bool? includeInactive,
  }) {
    final result = ListExceptionsRequest._();
    if (includeInactive != null) result.includeInactive = includeInactive;
    return result;
  }

  ListExceptionsRequest._();

  factory ListExceptionsRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListExceptionsRequest()..mergeFromBuffer(data, registry);
  factory ListExceptionsRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListExceptionsRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListExceptionsRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListExceptionsRequest.$_createMessage)
    ..aOB(1, _omitFieldNames ? '' : 'includeInactive')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListExceptionsRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListExceptionsRequest copyWith(
          void Function(ListExceptionsRequest) updates) =>
      super.copyWith((message) => updates(message as ListExceptionsRequest))
          as ListExceptionsRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use ListExceptionsRequest() / ListExceptionsRequest.new instead')
  static ListExceptionsRequest create() => ListExceptionsRequest._();
  static $pb.GeneratedMessage $_createMessage() => ListExceptionsRequest._();
  @$core.override
  ListExceptionsRequest createEmptyInstance() => ListExceptionsRequest._();
  @$core.pragma('dart2js:noInline')
  static ListExceptionsRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListExceptionsRequest>(
          ListExceptionsRequest.$_createMessage);
  static ListExceptionsRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.bool get includeInactive => $_getBF(0);
  @$pb.TagNumber(1)
  set includeInactive($core.bool value) => $_setBool(0, value);
  @$pb.TagNumber(1)
  $core.bool hasIncludeInactive() => $_has(0);
  @$pb.TagNumber(1)
  void clearIncludeInactive() => $_clearField(1);
}

class ListExceptionsResponse extends $pb.GeneratedMessage {
  factory ListExceptionsResponse({
    $core.Iterable<ExceptionRule>? rules,
  }) {
    final result = ListExceptionsResponse._();
    if (rules != null) result.rules.addAll(rules);
    return result;
  }

  ListExceptionsResponse._();

  factory ListExceptionsResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListExceptionsResponse()..mergeFromBuffer(data, registry);
  factory ListExceptionsResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListExceptionsResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListExceptionsResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListExceptionsResponse.$_createMessage)
    ..pPM<ExceptionRule>(1, _omitFieldNames ? '' : 'rules',
        subBuilder: ExceptionRule.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListExceptionsResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListExceptionsResponse copyWith(
          void Function(ListExceptionsResponse) updates) =>
      super.copyWith((message) => updates(message as ListExceptionsResponse))
          as ListExceptionsResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use ListExceptionsResponse() / ListExceptionsResponse.new instead')
  static ListExceptionsResponse create() => ListExceptionsResponse._();
  static $pb.GeneratedMessage $_createMessage() => ListExceptionsResponse._();
  @$core.override
  ListExceptionsResponse createEmptyInstance() => ListExceptionsResponse._();
  @$core.pragma('dart2js:noInline')
  static ListExceptionsResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListExceptionsResponse>(
          ListExceptionsResponse.$_createMessage);
  static ListExceptionsResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<ExceptionRule> get rules => $_getList(0);
}

class SaveExceptionRequest extends $pb.GeneratedMessage {
  factory SaveExceptionRequest({
    ExceptionRule? rule,
  }) {
    final result = SaveExceptionRequest._();
    if (rule != null) result.rule = rule;
    return result;
  }

  SaveExceptionRequest._();

  factory SaveExceptionRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      SaveExceptionRequest()..mergeFromBuffer(data, registry);
  factory SaveExceptionRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      SaveExceptionRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'SaveExceptionRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: SaveExceptionRequest.$_createMessage)
    ..aOM<ExceptionRule>(1, _omitFieldNames ? '' : 'rule',
        subBuilder: ExceptionRule.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SaveExceptionRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SaveExceptionRequest copyWith(void Function(SaveExceptionRequest) updates) =>
      super.copyWith((message) => updates(message as SaveExceptionRequest))
          as SaveExceptionRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use SaveExceptionRequest() / SaveExceptionRequest.new instead')
  static SaveExceptionRequest create() => SaveExceptionRequest._();
  static $pb.GeneratedMessage $_createMessage() => SaveExceptionRequest._();
  @$core.override
  SaveExceptionRequest createEmptyInstance() => SaveExceptionRequest._();
  @$core.pragma('dart2js:noInline')
  static SaveExceptionRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<SaveExceptionRequest>(
          SaveExceptionRequest.$_createMessage);
  static SaveExceptionRequest? _defaultInstance;

  @$pb.TagNumber(1)
  ExceptionRule get rule => $_getN(0);
  @$pb.TagNumber(1)
  set rule(ExceptionRule value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasRule() => $_has(0);
  @$pb.TagNumber(1)
  void clearRule() => $_clearField(1);
  @$pb.TagNumber(1)
  ExceptionRule ensureRule() => $_ensure(0);
}

class SaveExceptionResponse extends $pb.GeneratedMessage {
  factory SaveExceptionResponse({
    ExceptionRule? rule,
  }) {
    final result = SaveExceptionResponse._();
    if (rule != null) result.rule = rule;
    return result;
  }

  SaveExceptionResponse._();

  factory SaveExceptionResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      SaveExceptionResponse()..mergeFromBuffer(data, registry);
  factory SaveExceptionResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      SaveExceptionResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'SaveExceptionResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: SaveExceptionResponse.$_createMessage)
    ..aOM<ExceptionRule>(1, _omitFieldNames ? '' : 'rule',
        subBuilder: ExceptionRule.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SaveExceptionResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SaveExceptionResponse copyWith(
          void Function(SaveExceptionResponse) updates) =>
      super.copyWith((message) => updates(message as SaveExceptionResponse))
          as SaveExceptionResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use SaveExceptionResponse() / SaveExceptionResponse.new instead')
  static SaveExceptionResponse create() => SaveExceptionResponse._();
  static $pb.GeneratedMessage $_createMessage() => SaveExceptionResponse._();
  @$core.override
  SaveExceptionResponse createEmptyInstance() => SaveExceptionResponse._();
  @$core.pragma('dart2js:noInline')
  static SaveExceptionResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<SaveExceptionResponse>(
          SaveExceptionResponse.$_createMessage);
  static SaveExceptionResponse? _defaultInstance;

  @$pb.TagNumber(1)
  ExceptionRule get rule => $_getN(0);
  @$pb.TagNumber(1)
  set rule(ExceptionRule value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasRule() => $_has(0);
  @$pb.TagNumber(1)
  void clearRule() => $_clearField(1);
  @$pb.TagNumber(1)
  ExceptionRule ensureRule() => $_ensure(0);
}

class ExpireExceptionRequest extends $pb.GeneratedMessage {
  factory ExpireExceptionRequest({
    $core.String? id,
  }) {
    final result = ExpireExceptionRequest._();
    if (id != null) result.id = id;
    return result;
  }

  ExpireExceptionRequest._();

  factory ExpireExceptionRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ExpireExceptionRequest()..mergeFromBuffer(data, registry);
  factory ExpireExceptionRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ExpireExceptionRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ExpireExceptionRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ExpireExceptionRequest.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'id')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ExpireExceptionRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ExpireExceptionRequest copyWith(
          void Function(ExpireExceptionRequest) updates) =>
      super.copyWith((message) => updates(message as ExpireExceptionRequest))
          as ExpireExceptionRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use ExpireExceptionRequest() / ExpireExceptionRequest.new instead')
  static ExpireExceptionRequest create() => ExpireExceptionRequest._();
  static $pb.GeneratedMessage $_createMessage() => ExpireExceptionRequest._();
  @$core.override
  ExpireExceptionRequest createEmptyInstance() => ExpireExceptionRequest._();
  @$core.pragma('dart2js:noInline')
  static ExpireExceptionRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ExpireExceptionRequest>(
          ExpireExceptionRequest.$_createMessage);
  static ExpireExceptionRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get id => $_getSZ(0);
  @$pb.TagNumber(1)
  set id($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasId() => $_has(0);
  @$pb.TagNumber(1)
  void clearId() => $_clearField(1);
}

class ExpireExceptionResponse extends $pb.GeneratedMessage {
  factory ExpireExceptionResponse() => ExpireExceptionResponse._();

  ExpireExceptionResponse._();

  factory ExpireExceptionResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ExpireExceptionResponse()..mergeFromBuffer(data, registry);
  factory ExpireExceptionResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ExpireExceptionResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ExpireExceptionResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ExpireExceptionResponse.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ExpireExceptionResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ExpireExceptionResponse copyWith(
          void Function(ExpireExceptionResponse) updates) =>
      super.copyWith((message) => updates(message as ExpireExceptionResponse))
          as ExpireExceptionResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use ExpireExceptionResponse() / ExpireExceptionResponse.new instead')
  static ExpireExceptionResponse create() => ExpireExceptionResponse._();
  static $pb.GeneratedMessage $_createMessage() => ExpireExceptionResponse._();
  @$core.override
  ExpireExceptionResponse createEmptyInstance() => ExpireExceptionResponse._();
  @$core.pragma('dart2js:noInline')
  static ExpireExceptionResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ExpireExceptionResponse>(
          ExpireExceptionResponse.$_createMessage);
  static ExpireExceptionResponse? _defaultInstance;
}

/// Log browse and search
class SearchLogsRequest extends $pb.GeneratedMessage {
  factory SearchLogsRequest({
    $core.String? query,
    $core.String? machine,
    $fixnum.Int64? startTs,
    $fixnum.Int64? endTs,
    $core.int? limit,
    $core.String? category,
    $core.String? status,
    $core.int? offset,
  }) {
    final result = SearchLogsRequest._();
    if (query != null) result.query = query;
    if (machine != null) result.machine = machine;
    if (startTs != null) result.startTs = startTs;
    if (endTs != null) result.endTs = endTs;
    if (limit != null) result.limit = limit;
    if (category != null) result.category = category;
    if (status != null) result.status = status;
    if (offset != null) result.offset = offset;
    return result;
  }

  SearchLogsRequest._();

  factory SearchLogsRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      SearchLogsRequest()..mergeFromBuffer(data, registry);
  factory SearchLogsRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      SearchLogsRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'SearchLogsRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: SearchLogsRequest.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'query')
    ..aOS(2, _omitFieldNames ? '' : 'machine')
    ..aInt64(3, _omitFieldNames ? '' : 'startTs')
    ..aInt64(4, _omitFieldNames ? '' : 'endTs')
    ..aI(5, _omitFieldNames ? '' : 'limit')
    ..aOS(6, _omitFieldNames ? '' : 'category')
    ..aOS(7, _omitFieldNames ? '' : 'status')
    ..aI(8, _omitFieldNames ? '' : 'offset')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SearchLogsRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SearchLogsRequest copyWith(void Function(SearchLogsRequest) updates) =>
      super.copyWith((message) => updates(message as SearchLogsRequest))
          as SearchLogsRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use SearchLogsRequest() / SearchLogsRequest.new instead')
  static SearchLogsRequest create() => SearchLogsRequest._();
  static $pb.GeneratedMessage $_createMessage() => SearchLogsRequest._();
  @$core.override
  SearchLogsRequest createEmptyInstance() => SearchLogsRequest._();
  @$core.pragma('dart2js:noInline')
  static SearchLogsRequest getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<SearchLogsRequest>(
          SearchLogsRequest.$_createMessage);
  static SearchLogsRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get query => $_getSZ(0);
  @$pb.TagNumber(1)
  set query($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasQuery() => $_has(0);
  @$pb.TagNumber(1)
  void clearQuery() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get machine => $_getSZ(1);
  @$pb.TagNumber(2)
  set machine($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasMachine() => $_has(1);
  @$pb.TagNumber(2)
  void clearMachine() => $_clearField(2);

  @$pb.TagNumber(3)
  $fixnum.Int64 get startTs => $_getI64(2);
  @$pb.TagNumber(3)
  set startTs($fixnum.Int64 value) => $_setInt64(2, value);
  @$pb.TagNumber(3)
  $core.bool hasStartTs() => $_has(2);
  @$pb.TagNumber(3)
  void clearStartTs() => $_clearField(3);

  @$pb.TagNumber(4)
  $fixnum.Int64 get endTs => $_getI64(3);
  @$pb.TagNumber(4)
  set endTs($fixnum.Int64 value) => $_setInt64(3, value);
  @$pb.TagNumber(4)
  $core.bool hasEndTs() => $_has(3);
  @$pb.TagNumber(4)
  void clearEndTs() => $_clearField(4);

  @$pb.TagNumber(5)
  $core.int get limit => $_getIZ(4);
  @$pb.TagNumber(5)
  set limit($core.int value) => $_setSignedInt32(4, value);
  @$pb.TagNumber(5)
  $core.bool hasLimit() => $_has(4);
  @$pb.TagNumber(5)
  void clearLimit() => $_clearField(5);

  @$pb.TagNumber(6)
  $core.String get category => $_getSZ(5);
  @$pb.TagNumber(6)
  set category($core.String value) => $_setString(5, value);
  @$pb.TagNumber(6)
  $core.bool hasCategory() => $_has(5);
  @$pb.TagNumber(6)
  void clearCategory() => $_clearField(6);

  @$pb.TagNumber(7)
  $core.String get status => $_getSZ(6);
  @$pb.TagNumber(7)
  set status($core.String value) => $_setString(6, value);
  @$pb.TagNumber(7)
  $core.bool hasStatus() => $_has(6);
  @$pb.TagNumber(7)
  void clearStatus() => $_clearField(7);

  @$pb.TagNumber(8)
  $core.int get offset => $_getIZ(7);
  @$pb.TagNumber(8)
  set offset($core.int value) => $_setSignedInt32(7, value);
  @$pb.TagNumber(8)
  $core.bool hasOffset() => $_has(7);
  @$pb.TagNumber(8)
  void clearOffset() => $_clearField(8);
}

class SearchLogsResponse extends $pb.GeneratedMessage {
  factory SearchLogsResponse({
    $core.Iterable<$core.String>? lines,
    $core.int? total,
    $core.int? nextOffset,
  }) {
    final result = SearchLogsResponse._();
    if (lines != null) result.lines.addAll(lines);
    if (total != null) result.total = total;
    if (nextOffset != null) result.nextOffset = nextOffset;
    return result;
  }

  SearchLogsResponse._();

  factory SearchLogsResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      SearchLogsResponse()..mergeFromBuffer(data, registry);
  factory SearchLogsResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      SearchLogsResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'SearchLogsResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: SearchLogsResponse.$_createMessage)
    ..pPS(1, _omitFieldNames ? '' : 'lines')
    ..aI(2, _omitFieldNames ? '' : 'total')
    ..aI(3, _omitFieldNames ? '' : 'nextOffset')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SearchLogsResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SearchLogsResponse copyWith(void Function(SearchLogsResponse) updates) =>
      super.copyWith((message) => updates(message as SearchLogsResponse))
          as SearchLogsResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use SearchLogsResponse() / SearchLogsResponse.new instead')
  static SearchLogsResponse create() => SearchLogsResponse._();
  static $pb.GeneratedMessage $_createMessage() => SearchLogsResponse._();
  @$core.override
  SearchLogsResponse createEmptyInstance() => SearchLogsResponse._();
  @$core.pragma('dart2js:noInline')
  static SearchLogsResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<SearchLogsResponse>(
          SearchLogsResponse.$_createMessage);
  static SearchLogsResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<$core.String> get lines => $_getList(0);

  @$pb.TagNumber(2)
  $core.int get total => $_getIZ(1);
  @$pb.TagNumber(2)
  set total($core.int value) => $_setSignedInt32(1, value);
  @$pb.TagNumber(2)
  $core.bool hasTotal() => $_has(1);
  @$pb.TagNumber(2)
  void clearTotal() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.int get nextOffset => $_getIZ(2);
  @$pb.TagNumber(3)
  set nextOffset($core.int value) => $_setSignedInt32(2, value);
  @$pb.TagNumber(3)
  $core.bool hasNextOffset() => $_has(2);
  @$pb.TagNumber(3)
  void clearNextOffset() => $_clearField(3);
}

/// Audit log of mutating actions (on-demand runs, exception changes)
class AuditEntry extends $pb.GeneratedMessage {
  factory AuditEntry({
    $fixnum.Int64? ts,
    $core.String? actor,
    $core.String? action,
    $core.String? detail,
  }) {
    final result = AuditEntry._();
    if (ts != null) result.ts = ts;
    if (actor != null) result.actor = actor;
    if (action != null) result.action = action;
    if (detail != null) result.detail = detail;
    return result;
  }

  AuditEntry._();

  factory AuditEntry.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      AuditEntry()..mergeFromBuffer(data, registry);
  factory AuditEntry.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      AuditEntry()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'AuditEntry',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: AuditEntry.$_createMessage)
    ..aInt64(1, _omitFieldNames ? '' : 'ts')
    ..aOS(2, _omitFieldNames ? '' : 'actor')
    ..aOS(3, _omitFieldNames ? '' : 'action')
    ..aOS(4, _omitFieldNames ? '' : 'detail')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  AuditEntry clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  AuditEntry copyWith(void Function(AuditEntry) updates) =>
      super.copyWith((message) => updates(message as AuditEntry)) as AuditEntry;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use AuditEntry() / AuditEntry.new instead')
  static AuditEntry create() => AuditEntry._();
  static $pb.GeneratedMessage $_createMessage() => AuditEntry._();
  @$core.override
  AuditEntry createEmptyInstance() => AuditEntry._();
  @$core.pragma('dart2js:noInline')
  static AuditEntry getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<AuditEntry>(AuditEntry.$_createMessage);
  static AuditEntry? _defaultInstance;

  @$pb.TagNumber(1)
  $fixnum.Int64 get ts => $_getI64(0);
  @$pb.TagNumber(1)
  set ts($fixnum.Int64 value) => $_setInt64(0, value);
  @$pb.TagNumber(1)
  $core.bool hasTs() => $_has(0);
  @$pb.TagNumber(1)
  void clearTs() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get actor => $_getSZ(1);
  @$pb.TagNumber(2)
  set actor($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasActor() => $_has(1);
  @$pb.TagNumber(2)
  void clearActor() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get action => $_getSZ(2);
  @$pb.TagNumber(3)
  set action($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasAction() => $_has(2);
  @$pb.TagNumber(3)
  void clearAction() => $_clearField(3);

  @$pb.TagNumber(4)
  $core.String get detail => $_getSZ(3);
  @$pb.TagNumber(4)
  set detail($core.String value) => $_setString(3, value);
  @$pb.TagNumber(4)
  $core.bool hasDetail() => $_has(3);
  @$pb.TagNumber(4)
  void clearDetail() => $_clearField(4);
}

class ListAuditRequest extends $pb.GeneratedMessage {
  factory ListAuditRequest({
    $core.int? limit,
  }) {
    final result = ListAuditRequest._();
    if (limit != null) result.limit = limit;
    return result;
  }

  ListAuditRequest._();

  factory ListAuditRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListAuditRequest()..mergeFromBuffer(data, registry);
  factory ListAuditRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListAuditRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListAuditRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListAuditRequest.$_createMessage)
    ..aI(1, _omitFieldNames ? '' : 'limit')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListAuditRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListAuditRequest copyWith(void Function(ListAuditRequest) updates) =>
      super.copyWith((message) => updates(message as ListAuditRequest))
          as ListAuditRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use ListAuditRequest() / ListAuditRequest.new instead')
  static ListAuditRequest create() => ListAuditRequest._();
  static $pb.GeneratedMessage $_createMessage() => ListAuditRequest._();
  @$core.override
  ListAuditRequest createEmptyInstance() => ListAuditRequest._();
  @$core.pragma('dart2js:noInline')
  static ListAuditRequest getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<ListAuditRequest>(
          ListAuditRequest.$_createMessage);
  static ListAuditRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.int get limit => $_getIZ(0);
  @$pb.TagNumber(1)
  set limit($core.int value) => $_setSignedInt32(0, value);
  @$pb.TagNumber(1)
  $core.bool hasLimit() => $_has(0);
  @$pb.TagNumber(1)
  void clearLimit() => $_clearField(1);
}

class ListAuditResponse extends $pb.GeneratedMessage {
  factory ListAuditResponse({
    $core.Iterable<AuditEntry>? entries,
  }) {
    final result = ListAuditResponse._();
    if (entries != null) result.entries.addAll(entries);
    return result;
  }

  ListAuditResponse._();

  factory ListAuditResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListAuditResponse()..mergeFromBuffer(data, registry);
  factory ListAuditResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListAuditResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListAuditResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListAuditResponse.$_createMessage)
    ..pPM<AuditEntry>(1, _omitFieldNames ? '' : 'entries',
        subBuilder: AuditEntry.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListAuditResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListAuditResponse copyWith(void Function(ListAuditResponse) updates) =>
      super.copyWith((message) => updates(message as ListAuditResponse))
          as ListAuditResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use ListAuditResponse() / ListAuditResponse.new instead')
  static ListAuditResponse create() => ListAuditResponse._();
  static $pb.GeneratedMessage $_createMessage() => ListAuditResponse._();
  @$core.override
  ListAuditResponse createEmptyInstance() => ListAuditResponse._();
  @$core.pragma('dart2js:noInline')
  static ListAuditResponse getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<ListAuditResponse>(
          ListAuditResponse.$_createMessage);
  static ListAuditResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<AuditEntry> get entries => $_getList(0);
}

/// Live updates
class WatchUpdatesRequest extends $pb.GeneratedMessage {
  factory WatchUpdatesRequest() => WatchUpdatesRequest._();

  WatchUpdatesRequest._();

  factory WatchUpdatesRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      WatchUpdatesRequest()..mergeFromBuffer(data, registry);
  factory WatchUpdatesRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      WatchUpdatesRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'WatchUpdatesRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: WatchUpdatesRequest.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  WatchUpdatesRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  WatchUpdatesRequest copyWith(void Function(WatchUpdatesRequest) updates) =>
      super.copyWith((message) => updates(message as WatchUpdatesRequest))
          as WatchUpdatesRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core
      .Deprecated('Use WatchUpdatesRequest() / WatchUpdatesRequest.new instead')
  static WatchUpdatesRequest create() => WatchUpdatesRequest._();
  static $pb.GeneratedMessage $_createMessage() => WatchUpdatesRequest._();
  @$core.override
  WatchUpdatesRequest createEmptyInstance() => WatchUpdatesRequest._();
  @$core.pragma('dart2js:noInline')
  static WatchUpdatesRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<WatchUpdatesRequest>(
          WatchUpdatesRequest.$_createMessage);
  static WatchUpdatesRequest? _defaultInstance;
}

class Update extends $pb.GeneratedMessage {
  factory Update({
    $core.String? kind,
    $core.String? entity,
    $core.String? id,
    $fixnum.Int64? ts,
  }) {
    final result = Update._();
    if (kind != null) result.kind = kind;
    if (entity != null) result.entity = entity;
    if (id != null) result.id = id;
    if (ts != null) result.ts = ts;
    return result;
  }

  Update._();

  factory Update.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      Update()..mergeFromBuffer(data, registry);
  factory Update.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      Update()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'Update',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: Update.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'kind')
    ..aOS(2, _omitFieldNames ? '' : 'entity')
    ..aOS(3, _omitFieldNames ? '' : 'id')
    ..aInt64(4, _omitFieldNames ? '' : 'ts')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Update clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Update copyWith(void Function(Update) updates) =>
      super.copyWith((message) => updates(message as Update)) as Update;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use Update() / Update.new instead')
  static Update create() => Update._();
  static $pb.GeneratedMessage $_createMessage() => Update._();
  @$core.override
  Update createEmptyInstance() => Update._();
  @$core.pragma('dart2js:noInline')
  static Update getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<Update>(Update.$_createMessage);
  static Update? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get kind => $_getSZ(0);
  @$pb.TagNumber(1)
  set kind($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasKind() => $_has(0);
  @$pb.TagNumber(1)
  void clearKind() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get entity => $_getSZ(1);
  @$pb.TagNumber(2)
  set entity($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasEntity() => $_has(1);
  @$pb.TagNumber(2)
  void clearEntity() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get id => $_getSZ(2);
  @$pb.TagNumber(3)
  set id($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasId() => $_has(2);
  @$pb.TagNumber(3)
  void clearId() => $_clearField(3);

  @$pb.TagNumber(4)
  $fixnum.Int64 get ts => $_getI64(3);
  @$pb.TagNumber(4)
  set ts($fixnum.Int64 value) => $_setInt64(3, value);
  @$pb.TagNumber(4)
  $core.bool hasTs() => $_has(3);
  @$pb.TagNumber(4)
  void clearTs() => $_clearField(4);
}

/// Backups (decision 13). Owner/admin only, audit-logged. Restore is CLI-only (server stopped):
/// `itcmdr-cli restore --from ARCHIVE [--dry-run]`; app-triggered StageRestore is planned (decision 13).
class BackupInfo extends $pb.GeneratedMessage {
  factory BackupInfo({
    $core.String? name,
    $fixnum.Int64? size,
    $fixnum.Int64? createdTs,
    $core.String? kind,
    $core.String? itcmdrVersion,
    $fixnum.Int64? schemaVersion,
  }) {
    final result = BackupInfo._();
    if (name != null) result.name = name;
    if (size != null) result.size = size;
    if (createdTs != null) result.createdTs = createdTs;
    if (kind != null) result.kind = kind;
    if (itcmdrVersion != null) result.itcmdrVersion = itcmdrVersion;
    if (schemaVersion != null) result.schemaVersion = schemaVersion;
    return result;
  }

  BackupInfo._();

  factory BackupInfo.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      BackupInfo()..mergeFromBuffer(data, registry);
  factory BackupInfo.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      BackupInfo()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'BackupInfo',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: BackupInfo.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'name')
    ..aInt64(2, _omitFieldNames ? '' : 'size')
    ..aInt64(3, _omitFieldNames ? '' : 'createdTs')
    ..aOS(4, _omitFieldNames ? '' : 'kind')
    ..aOS(5, _omitFieldNames ? '' : 'itcmdrVersion')
    ..aInt64(6, _omitFieldNames ? '' : 'schemaVersion')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  BackupInfo clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  BackupInfo copyWith(void Function(BackupInfo) updates) =>
      super.copyWith((message) => updates(message as BackupInfo)) as BackupInfo;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use BackupInfo() / BackupInfo.new instead')
  static BackupInfo create() => BackupInfo._();
  static $pb.GeneratedMessage $_createMessage() => BackupInfo._();
  @$core.override
  BackupInfo createEmptyInstance() => BackupInfo._();
  @$core.pragma('dart2js:noInline')
  static BackupInfo getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<BackupInfo>(BackupInfo.$_createMessage);
  static BackupInfo? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get name => $_getSZ(0);
  @$pb.TagNumber(1)
  set name($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasName() => $_has(0);
  @$pb.TagNumber(1)
  void clearName() => $_clearField(1);

  @$pb.TagNumber(2)
  $fixnum.Int64 get size => $_getI64(1);
  @$pb.TagNumber(2)
  set size($fixnum.Int64 value) => $_setInt64(1, value);
  @$pb.TagNumber(2)
  $core.bool hasSize() => $_has(1);
  @$pb.TagNumber(2)
  void clearSize() => $_clearField(2);

  @$pb.TagNumber(3)
  $fixnum.Int64 get createdTs => $_getI64(2);
  @$pb.TagNumber(3)
  set createdTs($fixnum.Int64 value) => $_setInt64(2, value);
  @$pb.TagNumber(3)
  $core.bool hasCreatedTs() => $_has(2);
  @$pb.TagNumber(3)
  void clearCreatedTs() => $_clearField(3);

  @$pb.TagNumber(4)
  $core.String get kind => $_getSZ(3);
  @$pb.TagNumber(4)
  set kind($core.String value) => $_setString(3, value);
  @$pb.TagNumber(4)
  $core.bool hasKind() => $_has(3);
  @$pb.TagNumber(4)
  void clearKind() => $_clearField(4);

  @$pb.TagNumber(5)
  $core.String get itcmdrVersion => $_getSZ(4);
  @$pb.TagNumber(5)
  set itcmdrVersion($core.String value) => $_setString(4, value);
  @$pb.TagNumber(5)
  $core.bool hasItcmdrVersion() => $_has(4);
  @$pb.TagNumber(5)
  void clearItcmdrVersion() => $_clearField(5);

  @$pb.TagNumber(6)
  $fixnum.Int64 get schemaVersion => $_getI64(5);
  @$pb.TagNumber(6)
  set schemaVersion($fixnum.Int64 value) => $_setInt64(5, value);
  @$pb.TagNumber(6)
  $core.bool hasSchemaVersion() => $_has(5);
  @$pb.TagNumber(6)
  void clearSchemaVersion() => $_clearField(6);
}

class CreateBackupRequest extends $pb.GeneratedMessage {
  factory CreateBackupRequest() => CreateBackupRequest._();

  CreateBackupRequest._();

  factory CreateBackupRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      CreateBackupRequest()..mergeFromBuffer(data, registry);
  factory CreateBackupRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      CreateBackupRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'CreateBackupRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: CreateBackupRequest.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  CreateBackupRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  CreateBackupRequest copyWith(void Function(CreateBackupRequest) updates) =>
      super.copyWith((message) => updates(message as CreateBackupRequest))
          as CreateBackupRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core
      .Deprecated('Use CreateBackupRequest() / CreateBackupRequest.new instead')
  static CreateBackupRequest create() => CreateBackupRequest._();
  static $pb.GeneratedMessage $_createMessage() => CreateBackupRequest._();
  @$core.override
  CreateBackupRequest createEmptyInstance() => CreateBackupRequest._();
  @$core.pragma('dart2js:noInline')
  static CreateBackupRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<CreateBackupRequest>(
          CreateBackupRequest.$_createMessage);
  static CreateBackupRequest? _defaultInstance;
}

class CreateBackupResponse extends $pb.GeneratedMessage {
  factory CreateBackupResponse({
    BackupInfo? backup,
  }) {
    final result = CreateBackupResponse._();
    if (backup != null) result.backup = backup;
    return result;
  }

  CreateBackupResponse._();

  factory CreateBackupResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      CreateBackupResponse()..mergeFromBuffer(data, registry);
  factory CreateBackupResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      CreateBackupResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'CreateBackupResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: CreateBackupResponse.$_createMessage)
    ..aOM<BackupInfo>(1, _omitFieldNames ? '' : 'backup',
        subBuilder: BackupInfo.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  CreateBackupResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  CreateBackupResponse copyWith(void Function(CreateBackupResponse) updates) =>
      super.copyWith((message) => updates(message as CreateBackupResponse))
          as CreateBackupResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use CreateBackupResponse() / CreateBackupResponse.new instead')
  static CreateBackupResponse create() => CreateBackupResponse._();
  static $pb.GeneratedMessage $_createMessage() => CreateBackupResponse._();
  @$core.override
  CreateBackupResponse createEmptyInstance() => CreateBackupResponse._();
  @$core.pragma('dart2js:noInline')
  static CreateBackupResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<CreateBackupResponse>(
          CreateBackupResponse.$_createMessage);
  static CreateBackupResponse? _defaultInstance;

  @$pb.TagNumber(1)
  BackupInfo get backup => $_getN(0);
  @$pb.TagNumber(1)
  set backup(BackupInfo value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasBackup() => $_has(0);
  @$pb.TagNumber(1)
  void clearBackup() => $_clearField(1);
  @$pb.TagNumber(1)
  BackupInfo ensureBackup() => $_ensure(0);
}

class ListBackupsRequest extends $pb.GeneratedMessage {
  factory ListBackupsRequest() => ListBackupsRequest._();

  ListBackupsRequest._();

  factory ListBackupsRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListBackupsRequest()..mergeFromBuffer(data, registry);
  factory ListBackupsRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListBackupsRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListBackupsRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListBackupsRequest.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListBackupsRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListBackupsRequest copyWith(void Function(ListBackupsRequest) updates) =>
      super.copyWith((message) => updates(message as ListBackupsRequest))
          as ListBackupsRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use ListBackupsRequest() / ListBackupsRequest.new instead')
  static ListBackupsRequest create() => ListBackupsRequest._();
  static $pb.GeneratedMessage $_createMessage() => ListBackupsRequest._();
  @$core.override
  ListBackupsRequest createEmptyInstance() => ListBackupsRequest._();
  @$core.pragma('dart2js:noInline')
  static ListBackupsRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListBackupsRequest>(
          ListBackupsRequest.$_createMessage);
  static ListBackupsRequest? _defaultInstance;
}

class ListBackupsResponse extends $pb.GeneratedMessage {
  factory ListBackupsResponse({
    $core.Iterable<BackupInfo>? backups,
  }) {
    final result = ListBackupsResponse._();
    if (backups != null) result.backups.addAll(backups);
    return result;
  }

  ListBackupsResponse._();

  factory ListBackupsResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListBackupsResponse()..mergeFromBuffer(data, registry);
  factory ListBackupsResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListBackupsResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListBackupsResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListBackupsResponse.$_createMessage)
    ..pPM<BackupInfo>(1, _omitFieldNames ? '' : 'backups',
        subBuilder: BackupInfo.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListBackupsResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListBackupsResponse copyWith(void Function(ListBackupsResponse) updates) =>
      super.copyWith((message) => updates(message as ListBackupsResponse))
          as ListBackupsResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core
      .Deprecated('Use ListBackupsResponse() / ListBackupsResponse.new instead')
  static ListBackupsResponse create() => ListBackupsResponse._();
  static $pb.GeneratedMessage $_createMessage() => ListBackupsResponse._();
  @$core.override
  ListBackupsResponse createEmptyInstance() => ListBackupsResponse._();
  @$core.pragma('dart2js:noInline')
  static ListBackupsResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListBackupsResponse>(
          ListBackupsResponse.$_createMessage);
  static ListBackupsResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<BackupInfo> get backups => $_getList(0);
}

class VerifyBackupRequest extends $pb.GeneratedMessage {
  factory VerifyBackupRequest({
    $core.String? name,
  }) {
    final result = VerifyBackupRequest._();
    if (name != null) result.name = name;
    return result;
  }

  VerifyBackupRequest._();

  factory VerifyBackupRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      VerifyBackupRequest()..mergeFromBuffer(data, registry);
  factory VerifyBackupRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      VerifyBackupRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'VerifyBackupRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: VerifyBackupRequest.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'name')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  VerifyBackupRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  VerifyBackupRequest copyWith(void Function(VerifyBackupRequest) updates) =>
      super.copyWith((message) => updates(message as VerifyBackupRequest))
          as VerifyBackupRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core
      .Deprecated('Use VerifyBackupRequest() / VerifyBackupRequest.new instead')
  static VerifyBackupRequest create() => VerifyBackupRequest._();
  static $pb.GeneratedMessage $_createMessage() => VerifyBackupRequest._();
  @$core.override
  VerifyBackupRequest createEmptyInstance() => VerifyBackupRequest._();
  @$core.pragma('dart2js:noInline')
  static VerifyBackupRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<VerifyBackupRequest>(
          VerifyBackupRequest.$_createMessage);
  static VerifyBackupRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get name => $_getSZ(0);
  @$pb.TagNumber(1)
  set name($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasName() => $_has(0);
  @$pb.TagNumber(1)
  void clearName() => $_clearField(1);
}

class VerifyBackupResponse extends $pb.GeneratedMessage {
  factory VerifyBackupResponse({
    $core.bool? ok,
    $core.String? message,
  }) {
    final result = VerifyBackupResponse._();
    if (ok != null) result.ok = ok;
    if (message != null) result.message = message;
    return result;
  }

  VerifyBackupResponse._();

  factory VerifyBackupResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      VerifyBackupResponse()..mergeFromBuffer(data, registry);
  factory VerifyBackupResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      VerifyBackupResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'VerifyBackupResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: VerifyBackupResponse.$_createMessage)
    ..aOB(1, _omitFieldNames ? '' : 'ok')
    ..aOS(2, _omitFieldNames ? '' : 'message')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  VerifyBackupResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  VerifyBackupResponse copyWith(void Function(VerifyBackupResponse) updates) =>
      super.copyWith((message) => updates(message as VerifyBackupResponse))
          as VerifyBackupResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use VerifyBackupResponse() / VerifyBackupResponse.new instead')
  static VerifyBackupResponse create() => VerifyBackupResponse._();
  static $pb.GeneratedMessage $_createMessage() => VerifyBackupResponse._();
  @$core.override
  VerifyBackupResponse createEmptyInstance() => VerifyBackupResponse._();
  @$core.pragma('dart2js:noInline')
  static VerifyBackupResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<VerifyBackupResponse>(
          VerifyBackupResponse.$_createMessage);
  static VerifyBackupResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $core.bool get ok => $_getBF(0);
  @$pb.TagNumber(1)
  set ok($core.bool value) => $_setBool(0, value);
  @$pb.TagNumber(1)
  $core.bool hasOk() => $_has(0);
  @$pb.TagNumber(1)
  void clearOk() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get message => $_getSZ(1);
  @$pb.TagNumber(2)
  set message($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasMessage() => $_has(1);
  @$pb.TagNumber(2)
  void clearMessage() => $_clearField(2);
}

/// Settings (decision 15). Admin/owner only, audit-logged. Stored in the DB, override server.json.
class BackupSettings extends $pb.GeneratedMessage {
  factory BackupSettings({
    $core.String? dir,
    $core.bool? nightly,
    $core.int? hour,
    $core.int? keepDaily,
    $core.int? keepWeekly,
    $core.int? keepBlocks,
  }) {
    final result = BackupSettings._();
    if (dir != null) result.dir = dir;
    if (nightly != null) result.nightly = nightly;
    if (hour != null) result.hour = hour;
    if (keepDaily != null) result.keepDaily = keepDaily;
    if (keepWeekly != null) result.keepWeekly = keepWeekly;
    if (keepBlocks != null) result.keepBlocks = keepBlocks;
    return result;
  }

  BackupSettings._();

  factory BackupSettings.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      BackupSettings()..mergeFromBuffer(data, registry);
  factory BackupSettings.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      BackupSettings()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'BackupSettings',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: BackupSettings.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'dir')
    ..aOB(2, _omitFieldNames ? '' : 'nightly')
    ..aI(3, _omitFieldNames ? '' : 'hour', fieldType: $pb.PbFieldType.OU3)
    ..aI(4, _omitFieldNames ? '' : 'keepDaily', fieldType: $pb.PbFieldType.OU3)
    ..aI(5, _omitFieldNames ? '' : 'keepWeekly', fieldType: $pb.PbFieldType.OU3)
    ..aI(6, _omitFieldNames ? '' : 'keepBlocks', fieldType: $pb.PbFieldType.OU3)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  BackupSettings clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  BackupSettings copyWith(void Function(BackupSettings) updates) =>
      super.copyWith((message) => updates(message as BackupSettings))
          as BackupSettings;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use BackupSettings() / BackupSettings.new instead')
  static BackupSettings create() => BackupSettings._();
  static $pb.GeneratedMessage $_createMessage() => BackupSettings._();
  @$core.override
  BackupSettings createEmptyInstance() => BackupSettings._();
  @$core.pragma('dart2js:noInline')
  static BackupSettings getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<BackupSettings>(
          BackupSettings.$_createMessage);
  static BackupSettings? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get dir => $_getSZ(0);
  @$pb.TagNumber(1)
  set dir($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasDir() => $_has(0);
  @$pb.TagNumber(1)
  void clearDir() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.bool get nightly => $_getBF(1);
  @$pb.TagNumber(2)
  set nightly($core.bool value) => $_setBool(1, value);
  @$pb.TagNumber(2)
  $core.bool hasNightly() => $_has(1);
  @$pb.TagNumber(2)
  void clearNightly() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.int get hour => $_getIZ(2);
  @$pb.TagNumber(3)
  set hour($core.int value) => $_setUnsignedInt32(2, value);
  @$pb.TagNumber(3)
  $core.bool hasHour() => $_has(2);
  @$pb.TagNumber(3)
  void clearHour() => $_clearField(3);

  @$pb.TagNumber(4)
  $core.int get keepDaily => $_getIZ(3);
  @$pb.TagNumber(4)
  set keepDaily($core.int value) => $_setUnsignedInt32(3, value);
  @$pb.TagNumber(4)
  $core.bool hasKeepDaily() => $_has(3);
  @$pb.TagNumber(4)
  void clearKeepDaily() => $_clearField(4);

  @$pb.TagNumber(5)
  $core.int get keepWeekly => $_getIZ(4);
  @$pb.TagNumber(5)
  set keepWeekly($core.int value) => $_setUnsignedInt32(4, value);
  @$pb.TagNumber(5)
  $core.bool hasKeepWeekly() => $_has(4);
  @$pb.TagNumber(5)
  void clearKeepWeekly() => $_clearField(5);

  @$pb.TagNumber(6)
  $core.int get keepBlocks => $_getIZ(5);
  @$pb.TagNumber(6)
  set keepBlocks($core.int value) => $_setUnsignedInt32(5, value);
  @$pb.TagNumber(6)
  $core.bool hasKeepBlocks() => $_has(5);
  @$pb.TagNumber(6)
  void clearKeepBlocks() => $_clearField(6);
}

class GetSettingsRequest extends $pb.GeneratedMessage {
  factory GetSettingsRequest() => GetSettingsRequest._();

  GetSettingsRequest._();

  factory GetSettingsRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetSettingsRequest()..mergeFromBuffer(data, registry);
  factory GetSettingsRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetSettingsRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetSettingsRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: GetSettingsRequest.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetSettingsRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetSettingsRequest copyWith(void Function(GetSettingsRequest) updates) =>
      super.copyWith((message) => updates(message as GetSettingsRequest))
          as GetSettingsRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use GetSettingsRequest() / GetSettingsRequest.new instead')
  static GetSettingsRequest create() => GetSettingsRequest._();
  static $pb.GeneratedMessage $_createMessage() => GetSettingsRequest._();
  @$core.override
  GetSettingsRequest createEmptyInstance() => GetSettingsRequest._();
  @$core.pragma('dart2js:noInline')
  static GetSettingsRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetSettingsRequest>(
          GetSettingsRequest.$_createMessage);
  static GetSettingsRequest? _defaultInstance;
}

class GetSettingsResponse extends $pb.GeneratedMessage {
  factory GetSettingsResponse({
    BackupSettings? backup,
  }) {
    final result = GetSettingsResponse._();
    if (backup != null) result.backup = backup;
    return result;
  }

  GetSettingsResponse._();

  factory GetSettingsResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetSettingsResponse()..mergeFromBuffer(data, registry);
  factory GetSettingsResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetSettingsResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetSettingsResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: GetSettingsResponse.$_createMessage)
    ..aOM<BackupSettings>(1, _omitFieldNames ? '' : 'backup',
        subBuilder: BackupSettings.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetSettingsResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetSettingsResponse copyWith(void Function(GetSettingsResponse) updates) =>
      super.copyWith((message) => updates(message as GetSettingsResponse))
          as GetSettingsResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core
      .Deprecated('Use GetSettingsResponse() / GetSettingsResponse.new instead')
  static GetSettingsResponse create() => GetSettingsResponse._();
  static $pb.GeneratedMessage $_createMessage() => GetSettingsResponse._();
  @$core.override
  GetSettingsResponse createEmptyInstance() => GetSettingsResponse._();
  @$core.pragma('dart2js:noInline')
  static GetSettingsResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetSettingsResponse>(
          GetSettingsResponse.$_createMessage);
  static GetSettingsResponse? _defaultInstance;

  @$pb.TagNumber(1)
  BackupSettings get backup => $_getN(0);
  @$pb.TagNumber(1)
  set backup(BackupSettings value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasBackup() => $_has(0);
  @$pb.TagNumber(1)
  void clearBackup() => $_clearField(1);
  @$pb.TagNumber(1)
  BackupSettings ensureBackup() => $_ensure(0);
}

class UpdateSettingsRequest extends $pb.GeneratedMessage {
  factory UpdateSettingsRequest({
    BackupSettings? backup,
  }) {
    final result = UpdateSettingsRequest._();
    if (backup != null) result.backup = backup;
    return result;
  }

  UpdateSettingsRequest._();

  factory UpdateSettingsRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      UpdateSettingsRequest()..mergeFromBuffer(data, registry);
  factory UpdateSettingsRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      UpdateSettingsRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'UpdateSettingsRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: UpdateSettingsRequest.$_createMessage)
    ..aOM<BackupSettings>(1, _omitFieldNames ? '' : 'backup',
        subBuilder: BackupSettings.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UpdateSettingsRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UpdateSettingsRequest copyWith(
          void Function(UpdateSettingsRequest) updates) =>
      super.copyWith((message) => updates(message as UpdateSettingsRequest))
          as UpdateSettingsRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use UpdateSettingsRequest() / UpdateSettingsRequest.new instead')
  static UpdateSettingsRequest create() => UpdateSettingsRequest._();
  static $pb.GeneratedMessage $_createMessage() => UpdateSettingsRequest._();
  @$core.override
  UpdateSettingsRequest createEmptyInstance() => UpdateSettingsRequest._();
  @$core.pragma('dart2js:noInline')
  static UpdateSettingsRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<UpdateSettingsRequest>(
          UpdateSettingsRequest.$_createMessage);
  static UpdateSettingsRequest? _defaultInstance;

  @$pb.TagNumber(1)
  BackupSettings get backup => $_getN(0);
  @$pb.TagNumber(1)
  set backup(BackupSettings value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasBackup() => $_has(0);
  @$pb.TagNumber(1)
  void clearBackup() => $_clearField(1);
  @$pb.TagNumber(1)
  BackupSettings ensureBackup() => $_ensure(0);
}

class UpdateSettingsResponse extends $pb.GeneratedMessage {
  factory UpdateSettingsResponse({
    BackupSettings? backup,
  }) {
    final result = UpdateSettingsResponse._();
    if (backup != null) result.backup = backup;
    return result;
  }

  UpdateSettingsResponse._();

  factory UpdateSettingsResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      UpdateSettingsResponse()..mergeFromBuffer(data, registry);
  factory UpdateSettingsResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      UpdateSettingsResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'UpdateSettingsResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: UpdateSettingsResponse.$_createMessage)
    ..aOM<BackupSettings>(1, _omitFieldNames ? '' : 'backup',
        subBuilder: BackupSettings.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UpdateSettingsResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UpdateSettingsResponse copyWith(
          void Function(UpdateSettingsResponse) updates) =>
      super.copyWith((message) => updates(message as UpdateSettingsResponse))
          as UpdateSettingsResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use UpdateSettingsResponse() / UpdateSettingsResponse.new instead')
  static UpdateSettingsResponse create() => UpdateSettingsResponse._();
  static $pb.GeneratedMessage $_createMessage() => UpdateSettingsResponse._();
  @$core.override
  UpdateSettingsResponse createEmptyInstance() => UpdateSettingsResponse._();
  @$core.pragma('dart2js:noInline')
  static UpdateSettingsResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<UpdateSettingsResponse>(
          UpdateSettingsResponse.$_createMessage);
  static UpdateSettingsResponse? _defaultInstance;

  @$pb.TagNumber(1)
  BackupSettings get backup => $_getN(0);
  @$pb.TagNumber(1)
  set backup(BackupSettings value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasBackup() => $_has(0);
  @$pb.TagNumber(1)
  void clearBackup() => $_clearField(1);
  @$pb.TagNumber(1)
  BackupSettings ensureBackup() => $_ensure(0);
}

/// Every setting (decision 15, docs/SETTINGS.md). Admin/owner only; updates are validated and audited.
class SettingItem extends $pb.GeneratedMessage {
  factory SettingItem({
    $core.String? key,
    $core.String? scope,
    $core.String? type,
    $core.String? value,
    $core.String? defaultValue,
    $core.String? source,
    $core.String? description,
    $fixnum.Int64? min,
    $fixnum.Int64? max,
    $core.bool? readOnly,
    $core.String? applies,
  }) {
    final result = SettingItem._();
    if (key != null) result.key = key;
    if (scope != null) result.scope = scope;
    if (type != null) result.type = type;
    if (value != null) result.value = value;
    if (defaultValue != null) result.defaultValue = defaultValue;
    if (source != null) result.source = source;
    if (description != null) result.description = description;
    if (min != null) result.min = min;
    if (max != null) result.max = max;
    if (readOnly != null) result.readOnly = readOnly;
    if (applies != null) result.applies = applies;
    return result;
  }

  SettingItem._();

  factory SettingItem.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      SettingItem()..mergeFromBuffer(data, registry);
  factory SettingItem.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      SettingItem()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'SettingItem',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: SettingItem.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'key')
    ..aOS(2, _omitFieldNames ? '' : 'scope')
    ..aOS(3, _omitFieldNames ? '' : 'type')
    ..aOS(4, _omitFieldNames ? '' : 'value')
    ..aOS(5, _omitFieldNames ? '' : 'defaultValue')
    ..aOS(6, _omitFieldNames ? '' : 'source')
    ..aOS(7, _omitFieldNames ? '' : 'description')
    ..aInt64(8, _omitFieldNames ? '' : 'min')
    ..aInt64(9, _omitFieldNames ? '' : 'max')
    ..aOB(10, _omitFieldNames ? '' : 'readOnly')
    ..aOS(11, _omitFieldNames ? '' : 'applies')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SettingItem clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SettingItem copyWith(void Function(SettingItem) updates) =>
      super.copyWith((message) => updates(message as SettingItem))
          as SettingItem;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use SettingItem() / SettingItem.new instead')
  static SettingItem create() => SettingItem._();
  static $pb.GeneratedMessage $_createMessage() => SettingItem._();
  @$core.override
  SettingItem createEmptyInstance() => SettingItem._();
  @$core.pragma('dart2js:noInline')
  static SettingItem getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<SettingItem>(
          SettingItem.$_createMessage);
  static SettingItem? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get key => $_getSZ(0);
  @$pb.TagNumber(1)
  set key($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasKey() => $_has(0);
  @$pb.TagNumber(1)
  void clearKey() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get scope => $_getSZ(1);
  @$pb.TagNumber(2)
  set scope($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasScope() => $_has(1);
  @$pb.TagNumber(2)
  void clearScope() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get type => $_getSZ(2);
  @$pb.TagNumber(3)
  set type($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasType() => $_has(2);
  @$pb.TagNumber(3)
  void clearType() => $_clearField(3);

  @$pb.TagNumber(4)
  $core.String get value => $_getSZ(3);
  @$pb.TagNumber(4)
  set value($core.String value) => $_setString(3, value);
  @$pb.TagNumber(4)
  $core.bool hasValue() => $_has(3);
  @$pb.TagNumber(4)
  void clearValue() => $_clearField(4);

  @$pb.TagNumber(5)
  $core.String get defaultValue => $_getSZ(4);
  @$pb.TagNumber(5)
  set defaultValue($core.String value) => $_setString(4, value);
  @$pb.TagNumber(5)
  $core.bool hasDefaultValue() => $_has(4);
  @$pb.TagNumber(5)
  void clearDefaultValue() => $_clearField(5);

  @$pb.TagNumber(6)
  $core.String get source => $_getSZ(5);
  @$pb.TagNumber(6)
  set source($core.String value) => $_setString(5, value);
  @$pb.TagNumber(6)
  $core.bool hasSource() => $_has(5);
  @$pb.TagNumber(6)
  void clearSource() => $_clearField(6);

  @$pb.TagNumber(7)
  $core.String get description => $_getSZ(6);
  @$pb.TagNumber(7)
  set description($core.String value) => $_setString(6, value);
  @$pb.TagNumber(7)
  $core.bool hasDescription() => $_has(6);
  @$pb.TagNumber(7)
  void clearDescription() => $_clearField(7);

  @$pb.TagNumber(8)
  $fixnum.Int64 get min => $_getI64(7);
  @$pb.TagNumber(8)
  set min($fixnum.Int64 value) => $_setInt64(7, value);
  @$pb.TagNumber(8)
  $core.bool hasMin() => $_has(7);
  @$pb.TagNumber(8)
  void clearMin() => $_clearField(8);

  @$pb.TagNumber(9)
  $fixnum.Int64 get max => $_getI64(8);
  @$pb.TagNumber(9)
  set max($fixnum.Int64 value) => $_setInt64(8, value);
  @$pb.TagNumber(9)
  $core.bool hasMax() => $_has(8);
  @$pb.TagNumber(9)
  void clearMax() => $_clearField(9);

  @$pb.TagNumber(10)
  $core.bool get readOnly => $_getBF(9);
  @$pb.TagNumber(10)
  set readOnly($core.bool value) => $_setBool(9, value);
  @$pb.TagNumber(10)
  $core.bool hasReadOnly() => $_has(9);
  @$pb.TagNumber(10)
  void clearReadOnly() => $_clearField(10);

  @$pb.TagNumber(11)
  $core.String get applies => $_getSZ(10);
  @$pb.TagNumber(11)
  set applies($core.String value) => $_setString(10, value);
  @$pb.TagNumber(11)
  $core.bool hasApplies() => $_has(10);
  @$pb.TagNumber(11)
  void clearApplies() => $_clearField(11);
}

class ListSettingsRequest extends $pb.GeneratedMessage {
  factory ListSettingsRequest({
    $core.String? machine,
  }) {
    final result = ListSettingsRequest._();
    if (machine != null) result.machine = machine;
    return result;
  }

  ListSettingsRequest._();

  factory ListSettingsRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListSettingsRequest()..mergeFromBuffer(data, registry);
  factory ListSettingsRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListSettingsRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListSettingsRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListSettingsRequest.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'machine')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListSettingsRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListSettingsRequest copyWith(void Function(ListSettingsRequest) updates) =>
      super.copyWith((message) => updates(message as ListSettingsRequest))
          as ListSettingsRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core
      .Deprecated('Use ListSettingsRequest() / ListSettingsRequest.new instead')
  static ListSettingsRequest create() => ListSettingsRequest._();
  static $pb.GeneratedMessage $_createMessage() => ListSettingsRequest._();
  @$core.override
  ListSettingsRequest createEmptyInstance() => ListSettingsRequest._();
  @$core.pragma('dart2js:noInline')
  static ListSettingsRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListSettingsRequest>(
          ListSettingsRequest.$_createMessage);
  static ListSettingsRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get machine => $_getSZ(0);
  @$pb.TagNumber(1)
  set machine($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasMachine() => $_has(0);
  @$pb.TagNumber(1)
  void clearMachine() => $_clearField(1);
}

class ListSettingsResponse extends $pb.GeneratedMessage {
  factory ListSettingsResponse({
    $core.Iterable<SettingItem>? settings,
    $core.Iterable<$core.String>? machinesWithOverrides,
  }) {
    final result = ListSettingsResponse._();
    if (settings != null) result.settings.addAll(settings);
    if (machinesWithOverrides != null)
      result.machinesWithOverrides.addAll(machinesWithOverrides);
    return result;
  }

  ListSettingsResponse._();

  factory ListSettingsResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListSettingsResponse()..mergeFromBuffer(data, registry);
  factory ListSettingsResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListSettingsResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListSettingsResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListSettingsResponse.$_createMessage)
    ..pPM<SettingItem>(1, _omitFieldNames ? '' : 'settings',
        subBuilder: SettingItem.$_createMessage)
    ..pPS(2, _omitFieldNames ? '' : 'machinesWithOverrides')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListSettingsResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListSettingsResponse copyWith(void Function(ListSettingsResponse) updates) =>
      super.copyWith((message) => updates(message as ListSettingsResponse))
          as ListSettingsResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use ListSettingsResponse() / ListSettingsResponse.new instead')
  static ListSettingsResponse create() => ListSettingsResponse._();
  static $pb.GeneratedMessage $_createMessage() => ListSettingsResponse._();
  @$core.override
  ListSettingsResponse createEmptyInstance() => ListSettingsResponse._();
  @$core.pragma('dart2js:noInline')
  static ListSettingsResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListSettingsResponse>(
          ListSettingsResponse.$_createMessage);
  static ListSettingsResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<SettingItem> get settings => $_getList(0);

  @$pb.TagNumber(2)
  $pb.PbList<$core.String> get machinesWithOverrides => $_getList(1);
}

class UpdateSettingRequest extends $pb.GeneratedMessage {
  factory UpdateSettingRequest({
    $core.String? key,
    $core.String? value,
    $core.String? machine,
    $core.bool? unset,
  }) {
    final result = UpdateSettingRequest._();
    if (key != null) result.key = key;
    if (value != null) result.value = value;
    if (machine != null) result.machine = machine;
    if (unset != null) result.unset = unset;
    return result;
  }

  UpdateSettingRequest._();

  factory UpdateSettingRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      UpdateSettingRequest()..mergeFromBuffer(data, registry);
  factory UpdateSettingRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      UpdateSettingRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'UpdateSettingRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: UpdateSettingRequest.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'key')
    ..aOS(2, _omitFieldNames ? '' : 'value')
    ..aOS(3, _omitFieldNames ? '' : 'machine')
    ..aOB(4, _omitFieldNames ? '' : 'unset')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UpdateSettingRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UpdateSettingRequest copyWith(void Function(UpdateSettingRequest) updates) =>
      super.copyWith((message) => updates(message as UpdateSettingRequest))
          as UpdateSettingRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use UpdateSettingRequest() / UpdateSettingRequest.new instead')
  static UpdateSettingRequest create() => UpdateSettingRequest._();
  static $pb.GeneratedMessage $_createMessage() => UpdateSettingRequest._();
  @$core.override
  UpdateSettingRequest createEmptyInstance() => UpdateSettingRequest._();
  @$core.pragma('dart2js:noInline')
  static UpdateSettingRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<UpdateSettingRequest>(
          UpdateSettingRequest.$_createMessage);
  static UpdateSettingRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get key => $_getSZ(0);
  @$pb.TagNumber(1)
  set key($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasKey() => $_has(0);
  @$pb.TagNumber(1)
  void clearKey() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get value => $_getSZ(1);
  @$pb.TagNumber(2)
  set value($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasValue() => $_has(1);
  @$pb.TagNumber(2)
  void clearValue() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get machine => $_getSZ(2);
  @$pb.TagNumber(3)
  set machine($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasMachine() => $_has(2);
  @$pb.TagNumber(3)
  void clearMachine() => $_clearField(3);

  @$pb.TagNumber(4)
  $core.bool get unset => $_getBF(3);
  @$pb.TagNumber(4)
  set unset($core.bool value) => $_setBool(3, value);
  @$pb.TagNumber(4)
  $core.bool hasUnset() => $_has(3);
  @$pb.TagNumber(4)
  void clearUnset() => $_clearField(4);
}

class UpdateSettingResponse extends $pb.GeneratedMessage {
  factory UpdateSettingResponse({
    SettingItem? setting,
  }) {
    final result = UpdateSettingResponse._();
    if (setting != null) result.setting = setting;
    return result;
  }

  UpdateSettingResponse._();

  factory UpdateSettingResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      UpdateSettingResponse()..mergeFromBuffer(data, registry);
  factory UpdateSettingResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      UpdateSettingResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'UpdateSettingResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: UpdateSettingResponse.$_createMessage)
    ..aOM<SettingItem>(1, _omitFieldNames ? '' : 'setting',
        subBuilder: SettingItem.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UpdateSettingResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  UpdateSettingResponse copyWith(
          void Function(UpdateSettingResponse) updates) =>
      super.copyWith((message) => updates(message as UpdateSettingResponse))
          as UpdateSettingResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use UpdateSettingResponse() / UpdateSettingResponse.new instead')
  static UpdateSettingResponse create() => UpdateSettingResponse._();
  static $pb.GeneratedMessage $_createMessage() => UpdateSettingResponse._();
  @$core.override
  UpdateSettingResponse createEmptyInstance() => UpdateSettingResponse._();
  @$core.pragma('dart2js:noInline')
  static UpdateSettingResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<UpdateSettingResponse>(
          UpdateSettingResponse.$_createMessage);
  static UpdateSettingResponse? _defaultInstance;

  @$pb.TagNumber(1)
  SettingItem get setting => $_getN(0);
  @$pb.TagNumber(1)
  set setting(SettingItem value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasSetting() => $_has(0);
  @$pb.TagNumber(1)
  void clearSetting() => $_clearField(1);
  @$pb.TagNumber(1)
  SettingItem ensureSetting() => $_ensure(0);
}

/// ---- v0.10.0: plugin runtime (decisions 6, 11)
class RunPluginRequest extends $pb.GeneratedMessage {
  factory RunPluginRequest({
    $core.String? plugin,
    $core.bool? reportOnly,
    $core.Iterable<$core.MapEntry<$core.String, $core.String>>? args,
    $core.String? machine,
  }) {
    final result = RunPluginRequest._();
    if (plugin != null) result.plugin = plugin;
    if (reportOnly != null) result.reportOnly = reportOnly;
    if (args != null) result.args.addEntries(args);
    if (machine != null) result.machine = machine;
    return result;
  }

  RunPluginRequest._();

  factory RunPluginRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      RunPluginRequest()..mergeFromBuffer(data, registry);
  factory RunPluginRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      RunPluginRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'RunPluginRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: RunPluginRequest.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'plugin')
    ..aOB(2, _omitFieldNames ? '' : 'reportOnly')
    ..m<$core.String, $core.String>(3, _omitFieldNames ? '' : 'args',
        entryClassName: 'RunPluginRequest.ArgsEntry',
        keyFieldType: $pb.PbFieldType.OS,
        valueFieldType: $pb.PbFieldType.OS,
        packageName: const $pb.PackageName('itcmdr'))
    ..aOS(4, _omitFieldNames ? '' : 'machine')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  RunPluginRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  RunPluginRequest copyWith(void Function(RunPluginRequest) updates) =>
      super.copyWith((message) => updates(message as RunPluginRequest))
          as RunPluginRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use RunPluginRequest() / RunPluginRequest.new instead')
  static RunPluginRequest create() => RunPluginRequest._();
  static $pb.GeneratedMessage $_createMessage() => RunPluginRequest._();
  @$core.override
  RunPluginRequest createEmptyInstance() => RunPluginRequest._();
  @$core.pragma('dart2js:noInline')
  static RunPluginRequest getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<RunPluginRequest>(
          RunPluginRequest.$_createMessage);
  static RunPluginRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get plugin => $_getSZ(0);
  @$pb.TagNumber(1)
  set plugin($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasPlugin() => $_has(0);
  @$pb.TagNumber(1)
  void clearPlugin() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.bool get reportOnly => $_getBF(1);
  @$pb.TagNumber(2)
  set reportOnly($core.bool value) => $_setBool(1, value);
  @$pb.TagNumber(2)
  $core.bool hasReportOnly() => $_has(1);
  @$pb.TagNumber(2)
  void clearReportOnly() => $_clearField(2);

  @$pb.TagNumber(3)
  $pb.PbMap<$core.String, $core.String> get args => $_getMap(2);

  @$pb.TagNumber(4)
  $core.String get machine => $_getSZ(3);
  @$pb.TagNumber(4)
  set machine($core.String value) => $_setString(3, value);
  @$pb.TagNumber(4)
  $core.bool hasMachine() => $_has(3);
  @$pb.TagNumber(4)
  void clearMachine() => $_clearField(4);
}

class PluginStage extends $pb.GeneratedMessage {
  factory PluginStage({
    $core.String? stage,
    $core.bool? ok,
    $core.String? message,
  }) {
    final result = PluginStage._();
    if (stage != null) result.stage = stage;
    if (ok != null) result.ok = ok;
    if (message != null) result.message = message;
    return result;
  }

  PluginStage._();

  factory PluginStage.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      PluginStage()..mergeFromBuffer(data, registry);
  factory PluginStage.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      PluginStage()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'PluginStage',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: PluginStage.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'stage')
    ..aOB(2, _omitFieldNames ? '' : 'ok')
    ..aOS(3, _omitFieldNames ? '' : 'message')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  PluginStage clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  PluginStage copyWith(void Function(PluginStage) updates) =>
      super.copyWith((message) => updates(message as PluginStage))
          as PluginStage;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use PluginStage() / PluginStage.new instead')
  static PluginStage create() => PluginStage._();
  static $pb.GeneratedMessage $_createMessage() => PluginStage._();
  @$core.override
  PluginStage createEmptyInstance() => PluginStage._();
  @$core.pragma('dart2js:noInline')
  static PluginStage getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<PluginStage>(
          PluginStage.$_createMessage);
  static PluginStage? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get stage => $_getSZ(0);
  @$pb.TagNumber(1)
  set stage($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasStage() => $_has(0);
  @$pb.TagNumber(1)
  void clearStage() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.bool get ok => $_getBF(1);
  @$pb.TagNumber(2)
  set ok($core.bool value) => $_setBool(1, value);
  @$pb.TagNumber(2)
  $core.bool hasOk() => $_has(1);
  @$pb.TagNumber(2)
  void clearOk() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get message => $_getSZ(2);
  @$pb.TagNumber(3)
  set message($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasMessage() => $_has(2);
  @$pb.TagNumber(3)
  void clearMessage() => $_clearField(3);
}

class PluginCheckResult extends $pb.GeneratedMessage {
  factory PluginCheckResult({
    $core.String? checkId,
    $core.String? status,
    $core.String? message,
  }) {
    final result = PluginCheckResult._();
    if (checkId != null) result.checkId = checkId;
    if (status != null) result.status = status;
    if (message != null) result.message = message;
    return result;
  }

  PluginCheckResult._();

  factory PluginCheckResult.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      PluginCheckResult()..mergeFromBuffer(data, registry);
  factory PluginCheckResult.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      PluginCheckResult()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'PluginCheckResult',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: PluginCheckResult.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'checkId')
    ..aOS(2, _omitFieldNames ? '' : 'status')
    ..aOS(3, _omitFieldNames ? '' : 'message')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  PluginCheckResult clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  PluginCheckResult copyWith(void Function(PluginCheckResult) updates) =>
      super.copyWith((message) => updates(message as PluginCheckResult))
          as PluginCheckResult;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use PluginCheckResult() / PluginCheckResult.new instead')
  static PluginCheckResult create() => PluginCheckResult._();
  static $pb.GeneratedMessage $_createMessage() => PluginCheckResult._();
  @$core.override
  PluginCheckResult createEmptyInstance() => PluginCheckResult._();
  @$core.pragma('dart2js:noInline')
  static PluginCheckResult getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<PluginCheckResult>(
          PluginCheckResult.$_createMessage);
  static PluginCheckResult? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get checkId => $_getSZ(0);
  @$pb.TagNumber(1)
  set checkId($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasCheckId() => $_has(0);
  @$pb.TagNumber(1)
  void clearCheckId() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get status => $_getSZ(1);
  @$pb.TagNumber(2)
  set status($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasStatus() => $_has(1);
  @$pb.TagNumber(2)
  void clearStatus() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get message => $_getSZ(2);
  @$pb.TagNumber(3)
  set message($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasMessage() => $_has(2);
  @$pb.TagNumber(3)
  void clearMessage() => $_clearField(3);
}

class RunPluginResponse extends $pb.GeneratedMessage {
  factory RunPluginResponse({
    $core.String? runId,
    $core.String? status,
    $core.String? message,
    $core.Iterable<PluginStage>? stages,
    $core.Iterable<PluginCheckResult>? results,
  }) {
    final result = RunPluginResponse._();
    if (runId != null) result.runId = runId;
    if (status != null) result.status = status;
    if (message != null) result.message = message;
    if (stages != null) result.stages.addAll(stages);
    if (results != null) result.results.addAll(results);
    return result;
  }

  RunPluginResponse._();

  factory RunPluginResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      RunPluginResponse()..mergeFromBuffer(data, registry);
  factory RunPluginResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      RunPluginResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'RunPluginResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: RunPluginResponse.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'runId')
    ..aOS(2, _omitFieldNames ? '' : 'status')
    ..aOS(3, _omitFieldNames ? '' : 'message')
    ..pPM<PluginStage>(4, _omitFieldNames ? '' : 'stages',
        subBuilder: PluginStage.$_createMessage)
    ..pPM<PluginCheckResult>(5, _omitFieldNames ? '' : 'results',
        subBuilder: PluginCheckResult.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  RunPluginResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  RunPluginResponse copyWith(void Function(RunPluginResponse) updates) =>
      super.copyWith((message) => updates(message as RunPluginResponse))
          as RunPluginResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use RunPluginResponse() / RunPluginResponse.new instead')
  static RunPluginResponse create() => RunPluginResponse._();
  static $pb.GeneratedMessage $_createMessage() => RunPluginResponse._();
  @$core.override
  RunPluginResponse createEmptyInstance() => RunPluginResponse._();
  @$core.pragma('dart2js:noInline')
  static RunPluginResponse getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<RunPluginResponse>(
          RunPluginResponse.$_createMessage);
  static RunPluginResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get runId => $_getSZ(0);
  @$pb.TagNumber(1)
  set runId($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasRunId() => $_has(0);
  @$pb.TagNumber(1)
  void clearRunId() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get status => $_getSZ(1);
  @$pb.TagNumber(2)
  set status($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasStatus() => $_has(1);
  @$pb.TagNumber(2)
  void clearStatus() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get message => $_getSZ(2);
  @$pb.TagNumber(3)
  set message($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasMessage() => $_has(2);
  @$pb.TagNumber(3)
  void clearMessage() => $_clearField(3);

  @$pb.TagNumber(4)
  $pb.PbList<PluginStage> get stages => $_getList(3);

  @$pb.TagNumber(5)
  $pb.PbList<PluginCheckResult> get results => $_getList(4);
}

class ListPluginRunsRequest extends $pb.GeneratedMessage {
  factory ListPluginRunsRequest({
    $core.String? plugin,
    $core.int? limit,
  }) {
    final result = ListPluginRunsRequest._();
    if (plugin != null) result.plugin = plugin;
    if (limit != null) result.limit = limit;
    return result;
  }

  ListPluginRunsRequest._();

  factory ListPluginRunsRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListPluginRunsRequest()..mergeFromBuffer(data, registry);
  factory ListPluginRunsRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListPluginRunsRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListPluginRunsRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListPluginRunsRequest.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'plugin')
    ..aI(2, _omitFieldNames ? '' : 'limit')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListPluginRunsRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListPluginRunsRequest copyWith(
          void Function(ListPluginRunsRequest) updates) =>
      super.copyWith((message) => updates(message as ListPluginRunsRequest))
          as ListPluginRunsRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use ListPluginRunsRequest() / ListPluginRunsRequest.new instead')
  static ListPluginRunsRequest create() => ListPluginRunsRequest._();
  static $pb.GeneratedMessage $_createMessage() => ListPluginRunsRequest._();
  @$core.override
  ListPluginRunsRequest createEmptyInstance() => ListPluginRunsRequest._();
  @$core.pragma('dart2js:noInline')
  static ListPluginRunsRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListPluginRunsRequest>(
          ListPluginRunsRequest.$_createMessage);
  static ListPluginRunsRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get plugin => $_getSZ(0);
  @$pb.TagNumber(1)
  set plugin($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasPlugin() => $_has(0);
  @$pb.TagNumber(1)
  void clearPlugin() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.int get limit => $_getIZ(1);
  @$pb.TagNumber(2)
  set limit($core.int value) => $_setSignedInt32(1, value);
  @$pb.TagNumber(2)
  $core.bool hasLimit() => $_has(1);
  @$pb.TagNumber(2)
  void clearLimit() => $_clearField(2);
}

class PluginRun extends $pb.GeneratedMessage {
  factory PluginRun({
    $core.String? id,
    $core.String? plugin,
    $core.String? version,
    $core.String? machine,
    $fixnum.Int64? started,
    $fixnum.Int64? finished,
    $core.String? status,
    $core.String? message,
    $core.String? requestId,
  }) {
    final result = PluginRun._();
    if (id != null) result.id = id;
    if (plugin != null) result.plugin = plugin;
    if (version != null) result.version = version;
    if (machine != null) result.machine = machine;
    if (started != null) result.started = started;
    if (finished != null) result.finished = finished;
    if (status != null) result.status = status;
    if (message != null) result.message = message;
    if (requestId != null) result.requestId = requestId;
    return result;
  }

  PluginRun._();

  factory PluginRun.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      PluginRun()..mergeFromBuffer(data, registry);
  factory PluginRun.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      PluginRun()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'PluginRun',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: PluginRun.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'id')
    ..aOS(2, _omitFieldNames ? '' : 'plugin')
    ..aOS(3, _omitFieldNames ? '' : 'version')
    ..aOS(4, _omitFieldNames ? '' : 'machine')
    ..aInt64(5, _omitFieldNames ? '' : 'started')
    ..aInt64(6, _omitFieldNames ? '' : 'finished')
    ..aOS(7, _omitFieldNames ? '' : 'status')
    ..aOS(8, _omitFieldNames ? '' : 'message')
    ..aOS(9, _omitFieldNames ? '' : 'requestId')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  PluginRun clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  PluginRun copyWith(void Function(PluginRun) updates) =>
      super.copyWith((message) => updates(message as PluginRun)) as PluginRun;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use PluginRun() / PluginRun.new instead')
  static PluginRun create() => PluginRun._();
  static $pb.GeneratedMessage $_createMessage() => PluginRun._();
  @$core.override
  PluginRun createEmptyInstance() => PluginRun._();
  @$core.pragma('dart2js:noInline')
  static PluginRun getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<PluginRun>(PluginRun.$_createMessage);
  static PluginRun? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get id => $_getSZ(0);
  @$pb.TagNumber(1)
  set id($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasId() => $_has(0);
  @$pb.TagNumber(1)
  void clearId() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get plugin => $_getSZ(1);
  @$pb.TagNumber(2)
  set plugin($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasPlugin() => $_has(1);
  @$pb.TagNumber(2)
  void clearPlugin() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get version => $_getSZ(2);
  @$pb.TagNumber(3)
  set version($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasVersion() => $_has(2);
  @$pb.TagNumber(3)
  void clearVersion() => $_clearField(3);

  @$pb.TagNumber(4)
  $core.String get machine => $_getSZ(3);
  @$pb.TagNumber(4)
  set machine($core.String value) => $_setString(3, value);
  @$pb.TagNumber(4)
  $core.bool hasMachine() => $_has(3);
  @$pb.TagNumber(4)
  void clearMachine() => $_clearField(4);

  @$pb.TagNumber(5)
  $fixnum.Int64 get started => $_getI64(4);
  @$pb.TagNumber(5)
  set started($fixnum.Int64 value) => $_setInt64(4, value);
  @$pb.TagNumber(5)
  $core.bool hasStarted() => $_has(4);
  @$pb.TagNumber(5)
  void clearStarted() => $_clearField(5);

  @$pb.TagNumber(6)
  $fixnum.Int64 get finished => $_getI64(5);
  @$pb.TagNumber(6)
  set finished($fixnum.Int64 value) => $_setInt64(5, value);
  @$pb.TagNumber(6)
  $core.bool hasFinished() => $_has(5);
  @$pb.TagNumber(6)
  void clearFinished() => $_clearField(6);

  @$pb.TagNumber(7)
  $core.String get status => $_getSZ(6);
  @$pb.TagNumber(7)
  set status($core.String value) => $_setString(6, value);
  @$pb.TagNumber(7)
  $core.bool hasStatus() => $_has(6);
  @$pb.TagNumber(7)
  void clearStatus() => $_clearField(7);

  @$pb.TagNumber(8)
  $core.String get message => $_getSZ(7);
  @$pb.TagNumber(8)
  set message($core.String value) => $_setString(7, value);
  @$pb.TagNumber(8)
  $core.bool hasMessage() => $_has(7);
  @$pb.TagNumber(8)
  void clearMessage() => $_clearField(8);

  @$pb.TagNumber(9)
  $core.String get requestId => $_getSZ(8);
  @$pb.TagNumber(9)
  set requestId($core.String value) => $_setString(8, value);
  @$pb.TagNumber(9)
  $core.bool hasRequestId() => $_has(8);
  @$pb.TagNumber(9)
  void clearRequestId() => $_clearField(9);
}

class ListPluginRunsResponse extends $pb.GeneratedMessage {
  factory ListPluginRunsResponse({
    $core.Iterable<PluginRun>? runs,
  }) {
    final result = ListPluginRunsResponse._();
    if (runs != null) result.runs.addAll(runs);
    return result;
  }

  ListPluginRunsResponse._();

  factory ListPluginRunsResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListPluginRunsResponse()..mergeFromBuffer(data, registry);
  factory ListPluginRunsResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListPluginRunsResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListPluginRunsResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListPluginRunsResponse.$_createMessage)
    ..pPM<PluginRun>(1, _omitFieldNames ? '' : 'runs',
        subBuilder: PluginRun.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListPluginRunsResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListPluginRunsResponse copyWith(
          void Function(ListPluginRunsResponse) updates) =>
      super.copyWith((message) => updates(message as ListPluginRunsResponse))
          as ListPluginRunsResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use ListPluginRunsResponse() / ListPluginRunsResponse.new instead')
  static ListPluginRunsResponse create() => ListPluginRunsResponse._();
  static $pb.GeneratedMessage $_createMessage() => ListPluginRunsResponse._();
  @$core.override
  ListPluginRunsResponse createEmptyInstance() => ListPluginRunsResponse._();
  @$core.pragma('dart2js:noInline')
  static ListPluginRunsResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListPluginRunsResponse>(
          ListPluginRunsResponse.$_createMessage);
  static ListPluginRunsResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<PluginRun> get runs => $_getList(0);
}

class GetPluginMetricsRequest extends $pb.GeneratedMessage {
  factory GetPluginMetricsRequest({
    $core.String? name,
    $core.Iterable<$core.MapEntry<$core.String, $core.String>>? labels,
    $core.int? windowDays,
    $core.bool? daily,
  }) {
    final result = GetPluginMetricsRequest._();
    if (name != null) result.name = name;
    if (labels != null) result.labels.addEntries(labels);
    if (windowDays != null) result.windowDays = windowDays;
    if (daily != null) result.daily = daily;
    return result;
  }

  GetPluginMetricsRequest._();

  factory GetPluginMetricsRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetPluginMetricsRequest()..mergeFromBuffer(data, registry);
  factory GetPluginMetricsRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetPluginMetricsRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetPluginMetricsRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: GetPluginMetricsRequest.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'name')
    ..m<$core.String, $core.String>(2, _omitFieldNames ? '' : 'labels',
        entryClassName: 'GetPluginMetricsRequest.LabelsEntry',
        keyFieldType: $pb.PbFieldType.OS,
        valueFieldType: $pb.PbFieldType.OS,
        packageName: const $pb.PackageName('itcmdr'))
    ..aI(3, _omitFieldNames ? '' : 'windowDays')
    ..aOB(4, _omitFieldNames ? '' : 'daily')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetPluginMetricsRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetPluginMetricsRequest copyWith(
          void Function(GetPluginMetricsRequest) updates) =>
      super.copyWith((message) => updates(message as GetPluginMetricsRequest))
          as GetPluginMetricsRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use GetPluginMetricsRequest() / GetPluginMetricsRequest.new instead')
  static GetPluginMetricsRequest create() => GetPluginMetricsRequest._();
  static $pb.GeneratedMessage $_createMessage() => GetPluginMetricsRequest._();
  @$core.override
  GetPluginMetricsRequest createEmptyInstance() => GetPluginMetricsRequest._();
  @$core.pragma('dart2js:noInline')
  static GetPluginMetricsRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetPluginMetricsRequest>(
          GetPluginMetricsRequest.$_createMessage);
  static GetPluginMetricsRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get name => $_getSZ(0);
  @$pb.TagNumber(1)
  set name($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasName() => $_has(0);
  @$pb.TagNumber(1)
  void clearName() => $_clearField(1);

  @$pb.TagNumber(2)
  $pb.PbMap<$core.String, $core.String> get labels => $_getMap(1);

  @$pb.TagNumber(3)
  $core.int get windowDays => $_getIZ(2);
  @$pb.TagNumber(3)
  set windowDays($core.int value) => $_setSignedInt32(2, value);
  @$pb.TagNumber(3)
  $core.bool hasWindowDays() => $_has(2);
  @$pb.TagNumber(3)
  void clearWindowDays() => $_clearField(3);

  @$pb.TagNumber(4)
  $core.bool get daily => $_getBF(3);
  @$pb.TagNumber(4)
  set daily($core.bool value) => $_setBool(3, value);
  @$pb.TagNumber(4)
  $core.bool hasDaily() => $_has(3);
  @$pb.TagNumber(4)
  void clearDaily() => $_clearField(4);
}

class PluginMetricPoint extends $pb.GeneratedMessage {
  factory PluginMetricPoint({
    $core.String? name,
    $core.String? labels,
    $fixnum.Int64? ts,
    $core.double? value,
  }) {
    final result = PluginMetricPoint._();
    if (name != null) result.name = name;
    if (labels != null) result.labels = labels;
    if (ts != null) result.ts = ts;
    if (value != null) result.value = value;
    return result;
  }

  PluginMetricPoint._();

  factory PluginMetricPoint.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      PluginMetricPoint()..mergeFromBuffer(data, registry);
  factory PluginMetricPoint.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      PluginMetricPoint()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'PluginMetricPoint',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: PluginMetricPoint.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'name')
    ..aOS(2, _omitFieldNames ? '' : 'labels')
    ..aInt64(3, _omitFieldNames ? '' : 'ts')
    ..aD(4, _omitFieldNames ? '' : 'value')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  PluginMetricPoint clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  PluginMetricPoint copyWith(void Function(PluginMetricPoint) updates) =>
      super.copyWith((message) => updates(message as PluginMetricPoint))
          as PluginMetricPoint;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use PluginMetricPoint() / PluginMetricPoint.new instead')
  static PluginMetricPoint create() => PluginMetricPoint._();
  static $pb.GeneratedMessage $_createMessage() => PluginMetricPoint._();
  @$core.override
  PluginMetricPoint createEmptyInstance() => PluginMetricPoint._();
  @$core.pragma('dart2js:noInline')
  static PluginMetricPoint getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<PluginMetricPoint>(
          PluginMetricPoint.$_createMessage);
  static PluginMetricPoint? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get name => $_getSZ(0);
  @$pb.TagNumber(1)
  set name($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasName() => $_has(0);
  @$pb.TagNumber(1)
  void clearName() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get labels => $_getSZ(1);
  @$pb.TagNumber(2)
  set labels($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasLabels() => $_has(1);
  @$pb.TagNumber(2)
  void clearLabels() => $_clearField(2);

  @$pb.TagNumber(3)
  $fixnum.Int64 get ts => $_getI64(2);
  @$pb.TagNumber(3)
  set ts($fixnum.Int64 value) => $_setInt64(2, value);
  @$pb.TagNumber(3)
  $core.bool hasTs() => $_has(2);
  @$pb.TagNumber(3)
  void clearTs() => $_clearField(3);

  @$pb.TagNumber(4)
  $core.double get value => $_getN(3);
  @$pb.TagNumber(4)
  set value($core.double value) => $_setDouble(3, value);
  @$pb.TagNumber(4)
  $core.bool hasValue() => $_has(3);
  @$pb.TagNumber(4)
  void clearValue() => $_clearField(4);
}

class LogRejects extends $pb.GeneratedMessage {
  factory LogRejects({
    $core.String? file,
    $fixnum.Int64? count,
  }) {
    final result = LogRejects._();
    if (file != null) result.file = file;
    if (count != null) result.count = count;
    return result;
  }

  LogRejects._();

  factory LogRejects.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      LogRejects()..mergeFromBuffer(data, registry);
  factory LogRejects.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      LogRejects()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'LogRejects',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: LogRejects.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'file')
    ..aInt64(2, _omitFieldNames ? '' : 'count')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  LogRejects clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  LogRejects copyWith(void Function(LogRejects) updates) =>
      super.copyWith((message) => updates(message as LogRejects)) as LogRejects;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use LogRejects() / LogRejects.new instead')
  static LogRejects create() => LogRejects._();
  static $pb.GeneratedMessage $_createMessage() => LogRejects._();
  @$core.override
  LogRejects createEmptyInstance() => LogRejects._();
  @$core.pragma('dart2js:noInline')
  static LogRejects getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<LogRejects>(LogRejects.$_createMessage);
  static LogRejects? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get file => $_getSZ(0);
  @$pb.TagNumber(1)
  set file($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasFile() => $_has(0);
  @$pb.TagNumber(1)
  void clearFile() => $_clearField(1);

  @$pb.TagNumber(2)
  $fixnum.Int64 get count => $_getI64(1);
  @$pb.TagNumber(2)
  set count($fixnum.Int64 value) => $_setInt64(1, value);
  @$pb.TagNumber(2)
  $core.bool hasCount() => $_has(1);
  @$pb.TagNumber(2)
  void clearCount() => $_clearField(2);
}

class GetPluginMetricsResponse extends $pb.GeneratedMessage {
  factory GetPluginMetricsResponse({
    $core.Iterable<PluginMetricPoint>? points,
    $core.Iterable<LogRejects>? rejects,
  }) {
    final result = GetPluginMetricsResponse._();
    if (points != null) result.points.addAll(points);
    if (rejects != null) result.rejects.addAll(rejects);
    return result;
  }

  GetPluginMetricsResponse._();

  factory GetPluginMetricsResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetPluginMetricsResponse()..mergeFromBuffer(data, registry);
  factory GetPluginMetricsResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetPluginMetricsResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetPluginMetricsResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: GetPluginMetricsResponse.$_createMessage)
    ..pPM<PluginMetricPoint>(1, _omitFieldNames ? '' : 'points',
        subBuilder: PluginMetricPoint.$_createMessage)
    ..pPM<LogRejects>(2, _omitFieldNames ? '' : 'rejects',
        subBuilder: LogRejects.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetPluginMetricsResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetPluginMetricsResponse copyWith(
          void Function(GetPluginMetricsResponse) updates) =>
      super.copyWith((message) => updates(message as GetPluginMetricsResponse))
          as GetPluginMetricsResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use GetPluginMetricsResponse() / GetPluginMetricsResponse.new instead')
  static GetPluginMetricsResponse create() => GetPluginMetricsResponse._();
  static $pb.GeneratedMessage $_createMessage() => GetPluginMetricsResponse._();
  @$core.override
  GetPluginMetricsResponse createEmptyInstance() =>
      GetPluginMetricsResponse._();
  @$core.pragma('dart2js:noInline')
  static GetPluginMetricsResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetPluginMetricsResponse>(
          GetPluginMetricsResponse.$_createMessage);
  static GetPluginMetricsResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<PluginMetricPoint> get points => $_getList(0);

  @$pb.TagNumber(2)
  $pb.PbList<LogRejects> get rejects => $_getList(1);
}

/// ---- v0.10.0: schedules (decision 14). Admin manages; user may run now. All audited.
class Schedule extends $pb.GeneratedMessage {
  factory Schedule({
    $core.String? id,
    $core.String? name,
    $core.bool? enabled,
    $core.bool? allMachines,
    $core.Iterable<$core.String>? machines,
    $core.Iterable<$core.String>? groups,
    $core.Iterable<$core.String>? categories,
    $core.Iterable<$core.String>? plugins,
    $core.bool? repair,
    $core.String? when,
    $core.String? timezone,
    $core.String? guard,
    $core.String? missed,
    $core.bool? builtin,
    $core.Iterable<$fixnum.Int64>? nextRuns,
    $fixnum.Int64? lastFire,
    $core.String? builtinKey,
    $core.String? action,
  }) {
    final result = Schedule._();
    if (id != null) result.id = id;
    if (name != null) result.name = name;
    if (enabled != null) result.enabled = enabled;
    if (allMachines != null) result.allMachines = allMachines;
    if (machines != null) result.machines.addAll(machines);
    if (groups != null) result.groups.addAll(groups);
    if (categories != null) result.categories.addAll(categories);
    if (plugins != null) result.plugins.addAll(plugins);
    if (repair != null) result.repair = repair;
    if (when != null) result.when = when;
    if (timezone != null) result.timezone = timezone;
    if (guard != null) result.guard = guard;
    if (missed != null) result.missed = missed;
    if (builtin != null) result.builtin = builtin;
    if (nextRuns != null) result.nextRuns.addAll(nextRuns);
    if (lastFire != null) result.lastFire = lastFire;
    if (builtinKey != null) result.builtinKey = builtinKey;
    if (action != null) result.action = action;
    return result;
  }

  Schedule._();

  factory Schedule.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      Schedule()..mergeFromBuffer(data, registry);
  factory Schedule.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      Schedule()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'Schedule',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: Schedule.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'id')
    ..aOS(2, _omitFieldNames ? '' : 'name')
    ..aOB(3, _omitFieldNames ? '' : 'enabled')
    ..aOB(4, _omitFieldNames ? '' : 'allMachines')
    ..pPS(5, _omitFieldNames ? '' : 'machines')
    ..pPS(6, _omitFieldNames ? '' : 'groups')
    ..pPS(7, _omitFieldNames ? '' : 'categories')
    ..pPS(8, _omitFieldNames ? '' : 'plugins')
    ..aOB(9, _omitFieldNames ? '' : 'repair')
    ..aOS(10, _omitFieldNames ? '' : 'when')
    ..aOS(11, _omitFieldNames ? '' : 'timezone')
    ..aOS(12, _omitFieldNames ? '' : 'guard')
    ..aOS(13, _omitFieldNames ? '' : 'missed')
    ..aOB(14, _omitFieldNames ? '' : 'builtin')
    ..p<$fixnum.Int64>(
        15, _omitFieldNames ? '' : 'nextRuns', $pb.PbFieldType.K6)
    ..aInt64(16, _omitFieldNames ? '' : 'lastFire')
    ..aOS(17, _omitFieldNames ? '' : 'builtinKey')
    ..aOS(18, _omitFieldNames ? '' : 'action')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Schedule clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  Schedule copyWith(void Function(Schedule) updates) =>
      super.copyWith((message) => updates(message as Schedule)) as Schedule;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use Schedule() / Schedule.new instead')
  static Schedule create() => Schedule._();
  static $pb.GeneratedMessage $_createMessage() => Schedule._();
  @$core.override
  Schedule createEmptyInstance() => Schedule._();
  @$core.pragma('dart2js:noInline')
  static Schedule getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<Schedule>(Schedule.$_createMessage);
  static Schedule? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get id => $_getSZ(0);
  @$pb.TagNumber(1)
  set id($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasId() => $_has(0);
  @$pb.TagNumber(1)
  void clearId() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get name => $_getSZ(1);
  @$pb.TagNumber(2)
  set name($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasName() => $_has(1);
  @$pb.TagNumber(2)
  void clearName() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.bool get enabled => $_getBF(2);
  @$pb.TagNumber(3)
  set enabled($core.bool value) => $_setBool(2, value);
  @$pb.TagNumber(3)
  $core.bool hasEnabled() => $_has(2);
  @$pb.TagNumber(3)
  void clearEnabled() => $_clearField(3);

  @$pb.TagNumber(4)
  $core.bool get allMachines => $_getBF(3);
  @$pb.TagNumber(4)
  set allMachines($core.bool value) => $_setBool(3, value);
  @$pb.TagNumber(4)
  $core.bool hasAllMachines() => $_has(3);
  @$pb.TagNumber(4)
  void clearAllMachines() => $_clearField(4);

  @$pb.TagNumber(5)
  $pb.PbList<$core.String> get machines => $_getList(4);

  @$pb.TagNumber(6)
  $pb.PbList<$core.String> get groups => $_getList(5);

  @$pb.TagNumber(7)
  $pb.PbList<$core.String> get categories => $_getList(6);

  @$pb.TagNumber(8)
  $pb.PbList<$core.String> get plugins => $_getList(7);

  @$pb.TagNumber(9)
  $core.bool get repair => $_getBF(8);
  @$pb.TagNumber(9)
  set repair($core.bool value) => $_setBool(8, value);
  @$pb.TagNumber(9)
  $core.bool hasRepair() => $_has(8);
  @$pb.TagNumber(9)
  void clearRepair() => $_clearField(9);

  @$pb.TagNumber(10)
  $core.String get when => $_getSZ(9);
  @$pb.TagNumber(10)
  set when($core.String value) => $_setString(9, value);
  @$pb.TagNumber(10)
  $core.bool hasWhen() => $_has(9);
  @$pb.TagNumber(10)
  void clearWhen() => $_clearField(10);

  @$pb.TagNumber(11)
  $core.String get timezone => $_getSZ(10);
  @$pb.TagNumber(11)
  set timezone($core.String value) => $_setString(10, value);
  @$pb.TagNumber(11)
  $core.bool hasTimezone() => $_has(10);
  @$pb.TagNumber(11)
  void clearTimezone() => $_clearField(11);

  @$pb.TagNumber(12)
  $core.String get guard => $_getSZ(11);
  @$pb.TagNumber(12)
  set guard($core.String value) => $_setString(11, value);
  @$pb.TagNumber(12)
  $core.bool hasGuard() => $_has(11);
  @$pb.TagNumber(12)
  void clearGuard() => $_clearField(12);

  @$pb.TagNumber(13)
  $core.String get missed => $_getSZ(12);
  @$pb.TagNumber(13)
  set missed($core.String value) => $_setString(12, value);
  @$pb.TagNumber(13)
  $core.bool hasMissed() => $_has(12);
  @$pb.TagNumber(13)
  void clearMissed() => $_clearField(13);

  @$pb.TagNumber(14)
  $core.bool get builtin => $_getBF(13);
  @$pb.TagNumber(14)
  set builtin($core.bool value) => $_setBool(13, value);
  @$pb.TagNumber(14)
  $core.bool hasBuiltin() => $_has(13);
  @$pb.TagNumber(14)
  void clearBuiltin() => $_clearField(14);

  @$pb.TagNumber(15)
  $pb.PbList<$fixnum.Int64> get nextRuns => $_getList(14);

  @$pb.TagNumber(16)
  $fixnum.Int64 get lastFire => $_getI64(15);
  @$pb.TagNumber(16)
  set lastFire($fixnum.Int64 value) => $_setInt64(15, value);
  @$pb.TagNumber(16)
  $core.bool hasLastFire() => $_has(15);
  @$pb.TagNumber(16)
  void clearLastFire() => $_clearField(16);

  @$pb.TagNumber(17)
  $core.String get builtinKey => $_getSZ(16);
  @$pb.TagNumber(17)
  set builtinKey($core.String value) => $_setString(16, value);
  @$pb.TagNumber(17)
  $core.bool hasBuiltinKey() => $_has(16);
  @$pb.TagNumber(17)
  void clearBuiltinKey() => $_clearField(17);

  @$pb.TagNumber(18)
  $core.String get action => $_getSZ(17);
  @$pb.TagNumber(18)
  set action($core.String value) => $_setString(17, value);
  @$pb.TagNumber(18)
  $core.bool hasAction() => $_has(17);
  @$pb.TagNumber(18)
  void clearAction() => $_clearField(18);
}

class SaveScheduleRequest extends $pb.GeneratedMessage {
  factory SaveScheduleRequest({
    Schedule? schedule,
  }) {
    final result = SaveScheduleRequest._();
    if (schedule != null) result.schedule = schedule;
    return result;
  }

  SaveScheduleRequest._();

  factory SaveScheduleRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      SaveScheduleRequest()..mergeFromBuffer(data, registry);
  factory SaveScheduleRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      SaveScheduleRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'SaveScheduleRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: SaveScheduleRequest.$_createMessage)
    ..aOM<Schedule>(1, _omitFieldNames ? '' : 'schedule',
        subBuilder: Schedule.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SaveScheduleRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  SaveScheduleRequest copyWith(void Function(SaveScheduleRequest) updates) =>
      super.copyWith((message) => updates(message as SaveScheduleRequest))
          as SaveScheduleRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core
      .Deprecated('Use SaveScheduleRequest() / SaveScheduleRequest.new instead')
  static SaveScheduleRequest create() => SaveScheduleRequest._();
  static $pb.GeneratedMessage $_createMessage() => SaveScheduleRequest._();
  @$core.override
  SaveScheduleRequest createEmptyInstance() => SaveScheduleRequest._();
  @$core.pragma('dart2js:noInline')
  static SaveScheduleRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<SaveScheduleRequest>(
          SaveScheduleRequest.$_createMessage);
  static SaveScheduleRequest? _defaultInstance;

  @$pb.TagNumber(1)
  Schedule get schedule => $_getN(0);
  @$pb.TagNumber(1)
  set schedule(Schedule value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasSchedule() => $_has(0);
  @$pb.TagNumber(1)
  void clearSchedule() => $_clearField(1);
  @$pb.TagNumber(1)
  Schedule ensureSchedule() => $_ensure(0);
}

class ScheduleResponse extends $pb.GeneratedMessage {
  factory ScheduleResponse({
    Schedule? schedule,
  }) {
    final result = ScheduleResponse._();
    if (schedule != null) result.schedule = schedule;
    return result;
  }

  ScheduleResponse._();

  factory ScheduleResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ScheduleResponse()..mergeFromBuffer(data, registry);
  factory ScheduleResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ScheduleResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ScheduleResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ScheduleResponse.$_createMessage)
    ..aOM<Schedule>(1, _omitFieldNames ? '' : 'schedule',
        subBuilder: Schedule.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ScheduleResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ScheduleResponse copyWith(void Function(ScheduleResponse) updates) =>
      super.copyWith((message) => updates(message as ScheduleResponse))
          as ScheduleResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use ScheduleResponse() / ScheduleResponse.new instead')
  static ScheduleResponse create() => ScheduleResponse._();
  static $pb.GeneratedMessage $_createMessage() => ScheduleResponse._();
  @$core.override
  ScheduleResponse createEmptyInstance() => ScheduleResponse._();
  @$core.pragma('dart2js:noInline')
  static ScheduleResponse getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<ScheduleResponse>(
          ScheduleResponse.$_createMessage);
  static ScheduleResponse? _defaultInstance;

  @$pb.TagNumber(1)
  Schedule get schedule => $_getN(0);
  @$pb.TagNumber(1)
  set schedule(Schedule value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasSchedule() => $_has(0);
  @$pb.TagNumber(1)
  void clearSchedule() => $_clearField(1);
  @$pb.TagNumber(1)
  Schedule ensureSchedule() => $_ensure(0);
}

class ScheduleRef extends $pb.GeneratedMessage {
  factory ScheduleRef({
    $core.String? id,
    $core.bool? force,
    $core.String? confirm,
  }) {
    final result = ScheduleRef._();
    if (id != null) result.id = id;
    if (force != null) result.force = force;
    if (confirm != null) result.confirm = confirm;
    return result;
  }

  ScheduleRef._();

  factory ScheduleRef.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ScheduleRef()..mergeFromBuffer(data, registry);
  factory ScheduleRef.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ScheduleRef()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ScheduleRef',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ScheduleRef.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'id')
    ..aOB(2, _omitFieldNames ? '' : 'force')
    ..aOS(3, _omitFieldNames ? '' : 'confirm')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ScheduleRef clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ScheduleRef copyWith(void Function(ScheduleRef) updates) =>
      super.copyWith((message) => updates(message as ScheduleRef))
          as ScheduleRef;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use ScheduleRef() / ScheduleRef.new instead')
  static ScheduleRef create() => ScheduleRef._();
  static $pb.GeneratedMessage $_createMessage() => ScheduleRef._();
  @$core.override
  ScheduleRef createEmptyInstance() => ScheduleRef._();
  @$core.pragma('dart2js:noInline')
  static ScheduleRef getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<ScheduleRef>(
          ScheduleRef.$_createMessage);
  static ScheduleRef? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get id => $_getSZ(0);
  @$pb.TagNumber(1)
  set id($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasId() => $_has(0);
  @$pb.TagNumber(1)
  void clearId() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.bool get force => $_getBF(1);
  @$pb.TagNumber(2)
  set force($core.bool value) => $_setBool(1, value);
  @$pb.TagNumber(2)
  $core.bool hasForce() => $_has(1);
  @$pb.TagNumber(2)
  void clearForce() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get confirm => $_getSZ(2);
  @$pb.TagNumber(3)
  set confirm($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasConfirm() => $_has(2);
  @$pb.TagNumber(3)
  void clearConfirm() => $_clearField(3);
}

class ListSchedulesRequest extends $pb.GeneratedMessage {
  factory ListSchedulesRequest() => ListSchedulesRequest._();

  ListSchedulesRequest._();

  factory ListSchedulesRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListSchedulesRequest()..mergeFromBuffer(data, registry);
  factory ListSchedulesRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListSchedulesRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListSchedulesRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListSchedulesRequest.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListSchedulesRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListSchedulesRequest copyWith(void Function(ListSchedulesRequest) updates) =>
      super.copyWith((message) => updates(message as ListSchedulesRequest))
          as ListSchedulesRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use ListSchedulesRequest() / ListSchedulesRequest.new instead')
  static ListSchedulesRequest create() => ListSchedulesRequest._();
  static $pb.GeneratedMessage $_createMessage() => ListSchedulesRequest._();
  @$core.override
  ListSchedulesRequest createEmptyInstance() => ListSchedulesRequest._();
  @$core.pragma('dart2js:noInline')
  static ListSchedulesRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListSchedulesRequest>(
          ListSchedulesRequest.$_createMessage);
  static ListSchedulesRequest? _defaultInstance;
}

class ListSchedulesResponse extends $pb.GeneratedMessage {
  factory ListSchedulesResponse({
    $core.Iterable<Schedule>? schedules,
  }) {
    final result = ListSchedulesResponse._();
    if (schedules != null) result.schedules.addAll(schedules);
    return result;
  }

  ListSchedulesResponse._();

  factory ListSchedulesResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListSchedulesResponse()..mergeFromBuffer(data, registry);
  factory ListSchedulesResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListSchedulesResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListSchedulesResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListSchedulesResponse.$_createMessage)
    ..pPM<Schedule>(1, _omitFieldNames ? '' : 'schedules',
        subBuilder: Schedule.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListSchedulesResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListSchedulesResponse copyWith(
          void Function(ListSchedulesResponse) updates) =>
      super.copyWith((message) => updates(message as ListSchedulesResponse))
          as ListSchedulesResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use ListSchedulesResponse() / ListSchedulesResponse.new instead')
  static ListSchedulesResponse create() => ListSchedulesResponse._();
  static $pb.GeneratedMessage $_createMessage() => ListSchedulesResponse._();
  @$core.override
  ListSchedulesResponse createEmptyInstance() => ListSchedulesResponse._();
  @$core.pragma('dart2js:noInline')
  static ListSchedulesResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListSchedulesResponse>(
          ListSchedulesResponse.$_createMessage);
  static ListSchedulesResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<Schedule> get schedules => $_getList(0);
}

class RunScheduleNowResponse extends $pb.GeneratedMessage {
  factory RunScheduleNowResponse({
    $core.String? outcome,
    $core.String? requestId,
    $core.String? note,
  }) {
    final result = RunScheduleNowResponse._();
    if (outcome != null) result.outcome = outcome;
    if (requestId != null) result.requestId = requestId;
    if (note != null) result.note = note;
    return result;
  }

  RunScheduleNowResponse._();

  factory RunScheduleNowResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      RunScheduleNowResponse()..mergeFromBuffer(data, registry);
  factory RunScheduleNowResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      RunScheduleNowResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'RunScheduleNowResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: RunScheduleNowResponse.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'outcome')
    ..aOS(2, _omitFieldNames ? '' : 'requestId')
    ..aOS(3, _omitFieldNames ? '' : 'note')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  RunScheduleNowResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  RunScheduleNowResponse copyWith(
          void Function(RunScheduleNowResponse) updates) =>
      super.copyWith((message) => updates(message as RunScheduleNowResponse))
          as RunScheduleNowResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use RunScheduleNowResponse() / RunScheduleNowResponse.new instead')
  static RunScheduleNowResponse create() => RunScheduleNowResponse._();
  static $pb.GeneratedMessage $_createMessage() => RunScheduleNowResponse._();
  @$core.override
  RunScheduleNowResponse createEmptyInstance() => RunScheduleNowResponse._();
  @$core.pragma('dart2js:noInline')
  static RunScheduleNowResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<RunScheduleNowResponse>(
          RunScheduleNowResponse.$_createMessage);
  static RunScheduleNowResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get outcome => $_getSZ(0);
  @$pb.TagNumber(1)
  set outcome($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasOutcome() => $_has(0);
  @$pb.TagNumber(1)
  void clearOutcome() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get requestId => $_getSZ(1);
  @$pb.TagNumber(2)
  set requestId($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasRequestId() => $_has(1);
  @$pb.TagNumber(2)
  void clearRequestId() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get note => $_getSZ(2);
  @$pb.TagNumber(3)
  set note($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasNote() => $_has(2);
  @$pb.TagNumber(3)
  void clearNote() => $_clearField(3);
}

class ListScheduleRunsRequest extends $pb.GeneratedMessage {
  factory ListScheduleRunsRequest({
    $core.String? id,
    $core.int? limit,
  }) {
    final result = ListScheduleRunsRequest._();
    if (id != null) result.id = id;
    if (limit != null) result.limit = limit;
    return result;
  }

  ListScheduleRunsRequest._();

  factory ListScheduleRunsRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListScheduleRunsRequest()..mergeFromBuffer(data, registry);
  factory ListScheduleRunsRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListScheduleRunsRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListScheduleRunsRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListScheduleRunsRequest.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'id')
    ..aI(2, _omitFieldNames ? '' : 'limit')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListScheduleRunsRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListScheduleRunsRequest copyWith(
          void Function(ListScheduleRunsRequest) updates) =>
      super.copyWith((message) => updates(message as ListScheduleRunsRequest))
          as ListScheduleRunsRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use ListScheduleRunsRequest() / ListScheduleRunsRequest.new instead')
  static ListScheduleRunsRequest create() => ListScheduleRunsRequest._();
  static $pb.GeneratedMessage $_createMessage() => ListScheduleRunsRequest._();
  @$core.override
  ListScheduleRunsRequest createEmptyInstance() => ListScheduleRunsRequest._();
  @$core.pragma('dart2js:noInline')
  static ListScheduleRunsRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListScheduleRunsRequest>(
          ListScheduleRunsRequest.$_createMessage);
  static ListScheduleRunsRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get id => $_getSZ(0);
  @$pb.TagNumber(1)
  set id($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasId() => $_has(0);
  @$pb.TagNumber(1)
  void clearId() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.int get limit => $_getIZ(1);
  @$pb.TagNumber(2)
  set limit($core.int value) => $_setSignedInt32(1, value);
  @$pb.TagNumber(2)
  $core.bool hasLimit() => $_has(1);
  @$pb.TagNumber(2)
  void clearLimit() => $_clearField(2);
}

class ScheduleRun extends $pb.GeneratedMessage {
  factory ScheduleRun({
    $core.String? scheduleId,
    $fixnum.Int64? ts,
    $fixnum.Int64? due,
    $core.String? requestId,
    $core.String? outcome,
    $core.String? note,
  }) {
    final result = ScheduleRun._();
    if (scheduleId != null) result.scheduleId = scheduleId;
    if (ts != null) result.ts = ts;
    if (due != null) result.due = due;
    if (requestId != null) result.requestId = requestId;
    if (outcome != null) result.outcome = outcome;
    if (note != null) result.note = note;
    return result;
  }

  ScheduleRun._();

  factory ScheduleRun.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ScheduleRun()..mergeFromBuffer(data, registry);
  factory ScheduleRun.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ScheduleRun()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ScheduleRun',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ScheduleRun.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'scheduleId')
    ..aInt64(2, _omitFieldNames ? '' : 'ts')
    ..aInt64(3, _omitFieldNames ? '' : 'due')
    ..aOS(4, _omitFieldNames ? '' : 'requestId')
    ..aOS(5, _omitFieldNames ? '' : 'outcome')
    ..aOS(6, _omitFieldNames ? '' : 'note')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ScheduleRun clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ScheduleRun copyWith(void Function(ScheduleRun) updates) =>
      super.copyWith((message) => updates(message as ScheduleRun))
          as ScheduleRun;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use ScheduleRun() / ScheduleRun.new instead')
  static ScheduleRun create() => ScheduleRun._();
  static $pb.GeneratedMessage $_createMessage() => ScheduleRun._();
  @$core.override
  ScheduleRun createEmptyInstance() => ScheduleRun._();
  @$core.pragma('dart2js:noInline')
  static ScheduleRun getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<ScheduleRun>(
          ScheduleRun.$_createMessage);
  static ScheduleRun? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get scheduleId => $_getSZ(0);
  @$pb.TagNumber(1)
  set scheduleId($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasScheduleId() => $_has(0);
  @$pb.TagNumber(1)
  void clearScheduleId() => $_clearField(1);

  @$pb.TagNumber(2)
  $fixnum.Int64 get ts => $_getI64(1);
  @$pb.TagNumber(2)
  set ts($fixnum.Int64 value) => $_setInt64(1, value);
  @$pb.TagNumber(2)
  $core.bool hasTs() => $_has(1);
  @$pb.TagNumber(2)
  void clearTs() => $_clearField(2);

  @$pb.TagNumber(3)
  $fixnum.Int64 get due => $_getI64(2);
  @$pb.TagNumber(3)
  set due($fixnum.Int64 value) => $_setInt64(2, value);
  @$pb.TagNumber(3)
  $core.bool hasDue() => $_has(2);
  @$pb.TagNumber(3)
  void clearDue() => $_clearField(3);

  @$pb.TagNumber(4)
  $core.String get requestId => $_getSZ(3);
  @$pb.TagNumber(4)
  set requestId($core.String value) => $_setString(3, value);
  @$pb.TagNumber(4)
  $core.bool hasRequestId() => $_has(3);
  @$pb.TagNumber(4)
  void clearRequestId() => $_clearField(4);

  @$pb.TagNumber(5)
  $core.String get outcome => $_getSZ(4);
  @$pb.TagNumber(5)
  set outcome($core.String value) => $_setString(4, value);
  @$pb.TagNumber(5)
  $core.bool hasOutcome() => $_has(4);
  @$pb.TagNumber(5)
  void clearOutcome() => $_clearField(5);

  @$pb.TagNumber(6)
  $core.String get note => $_getSZ(5);
  @$pb.TagNumber(6)
  set note($core.String value) => $_setString(5, value);
  @$pb.TagNumber(6)
  $core.bool hasNote() => $_has(5);
  @$pb.TagNumber(6)
  void clearNote() => $_clearField(6);
}

class ListScheduleRunsResponse extends $pb.GeneratedMessage {
  factory ListScheduleRunsResponse({
    $core.Iterable<ScheduleRun>? runs,
  }) {
    final result = ListScheduleRunsResponse._();
    if (runs != null) result.runs.addAll(runs);
    return result;
  }

  ListScheduleRunsResponse._();

  factory ListScheduleRunsResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListScheduleRunsResponse()..mergeFromBuffer(data, registry);
  factory ListScheduleRunsResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      ListScheduleRunsResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'ListScheduleRunsResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: ListScheduleRunsResponse.$_createMessage)
    ..pPM<ScheduleRun>(1, _omitFieldNames ? '' : 'runs',
        subBuilder: ScheduleRun.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListScheduleRunsResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  ListScheduleRunsResponse copyWith(
          void Function(ListScheduleRunsResponse) updates) =>
      super.copyWith((message) => updates(message as ListScheduleRunsResponse))
          as ListScheduleRunsResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use ListScheduleRunsResponse() / ListScheduleRunsResponse.new instead')
  static ListScheduleRunsResponse create() => ListScheduleRunsResponse._();
  static $pb.GeneratedMessage $_createMessage() => ListScheduleRunsResponse._();
  @$core.override
  ListScheduleRunsResponse createEmptyInstance() =>
      ListScheduleRunsResponse._();
  @$core.pragma('dart2js:noInline')
  static ListScheduleRunsResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<ListScheduleRunsResponse>(
          ListScheduleRunsResponse.$_createMessage);
  static ListScheduleRunsResponse? _defaultInstance;

  @$pb.TagNumber(1)
  $pb.PbList<ScheduleRun> get runs => $_getList(0);
}

/// ---- v0.10.0: app-triggered restore (decision 13). Owner only, audited.
class StageRestoreRequest extends $pb.GeneratedMessage {
  factory StageRestoreRequest({
    $core.String? backup,
    $core.String? words,
  }) {
    final result = StageRestoreRequest._();
    if (backup != null) result.backup = backup;
    if (words != null) result.words = words;
    return result;
  }

  StageRestoreRequest._();

  factory StageRestoreRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      StageRestoreRequest()..mergeFromBuffer(data, registry);
  factory StageRestoreRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      StageRestoreRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'StageRestoreRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: StageRestoreRequest.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'backup')
    ..aOS(2, _omitFieldNames ? '' : 'words')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StageRestoreRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StageRestoreRequest copyWith(void Function(StageRestoreRequest) updates) =>
      super.copyWith((message) => updates(message as StageRestoreRequest))
          as StageRestoreRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core
      .Deprecated('Use StageRestoreRequest() / StageRestoreRequest.new instead')
  static StageRestoreRequest create() => StageRestoreRequest._();
  static $pb.GeneratedMessage $_createMessage() => StageRestoreRequest._();
  @$core.override
  StageRestoreRequest createEmptyInstance() => StageRestoreRequest._();
  @$core.pragma('dart2js:noInline')
  static StageRestoreRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<StageRestoreRequest>(
          StageRestoreRequest.$_createMessage);
  static StageRestoreRequest? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get backup => $_getSZ(0);
  @$pb.TagNumber(1)
  set backup($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasBackup() => $_has(0);
  @$pb.TagNumber(1)
  void clearBackup() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get words => $_getSZ(1);
  @$pb.TagNumber(2)
  set words($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasWords() => $_has(1);
  @$pb.TagNumber(2)
  void clearWords() => $_clearField(2);
}

class RestoreStatus extends $pb.GeneratedMessage {
  factory RestoreStatus({
    $core.String? state,
    $core.String? message,
    $core.String? backup,
    $core.String? actor,
    $fixnum.Int64? started,
    $fixnum.Int64? updated,
    $core.String? safetyCopy,
  }) {
    final result = RestoreStatus._();
    if (state != null) result.state = state;
    if (message != null) result.message = message;
    if (backup != null) result.backup = backup;
    if (actor != null) result.actor = actor;
    if (started != null) result.started = started;
    if (updated != null) result.updated = updated;
    if (safetyCopy != null) result.safetyCopy = safetyCopy;
    return result;
  }

  RestoreStatus._();

  factory RestoreStatus.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      RestoreStatus()..mergeFromBuffer(data, registry);
  factory RestoreStatus.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      RestoreStatus()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'RestoreStatus',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: RestoreStatus.$_createMessage)
    ..aOS(1, _omitFieldNames ? '' : 'state')
    ..aOS(2, _omitFieldNames ? '' : 'message')
    ..aOS(3, _omitFieldNames ? '' : 'backup')
    ..aOS(4, _omitFieldNames ? '' : 'actor')
    ..aInt64(5, _omitFieldNames ? '' : 'started')
    ..aInt64(6, _omitFieldNames ? '' : 'updated')
    ..aOS(7, _omitFieldNames ? '' : 'safetyCopy')
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  RestoreStatus clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  RestoreStatus copyWith(void Function(RestoreStatus) updates) =>
      super.copyWith((message) => updates(message as RestoreStatus))
          as RestoreStatus;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated('Use RestoreStatus() / RestoreStatus.new instead')
  static RestoreStatus create() => RestoreStatus._();
  static $pb.GeneratedMessage $_createMessage() => RestoreStatus._();
  @$core.override
  RestoreStatus createEmptyInstance() => RestoreStatus._();
  @$core.pragma('dart2js:noInline')
  static RestoreStatus getDefault() =>
      _defaultInstance ??= $pb.GeneratedMessage.$_defaultFor<RestoreStatus>(
          RestoreStatus.$_createMessage);
  static RestoreStatus? _defaultInstance;

  @$pb.TagNumber(1)
  $core.String get state => $_getSZ(0);
  @$pb.TagNumber(1)
  set state($core.String value) => $_setString(0, value);
  @$pb.TagNumber(1)
  $core.bool hasState() => $_has(0);
  @$pb.TagNumber(1)
  void clearState() => $_clearField(1);

  @$pb.TagNumber(2)
  $core.String get message => $_getSZ(1);
  @$pb.TagNumber(2)
  set message($core.String value) => $_setString(1, value);
  @$pb.TagNumber(2)
  $core.bool hasMessage() => $_has(1);
  @$pb.TagNumber(2)
  void clearMessage() => $_clearField(2);

  @$pb.TagNumber(3)
  $core.String get backup => $_getSZ(2);
  @$pb.TagNumber(3)
  set backup($core.String value) => $_setString(2, value);
  @$pb.TagNumber(3)
  $core.bool hasBackup() => $_has(2);
  @$pb.TagNumber(3)
  void clearBackup() => $_clearField(3);

  @$pb.TagNumber(4)
  $core.String get actor => $_getSZ(3);
  @$pb.TagNumber(4)
  set actor($core.String value) => $_setString(3, value);
  @$pb.TagNumber(4)
  $core.bool hasActor() => $_has(3);
  @$pb.TagNumber(4)
  void clearActor() => $_clearField(4);

  @$pb.TagNumber(5)
  $fixnum.Int64 get started => $_getI64(4);
  @$pb.TagNumber(5)
  set started($fixnum.Int64 value) => $_setInt64(4, value);
  @$pb.TagNumber(5)
  $core.bool hasStarted() => $_has(4);
  @$pb.TagNumber(5)
  void clearStarted() => $_clearField(5);

  @$pb.TagNumber(6)
  $fixnum.Int64 get updated => $_getI64(5);
  @$pb.TagNumber(6)
  set updated($fixnum.Int64 value) => $_setInt64(5, value);
  @$pb.TagNumber(6)
  $core.bool hasUpdated() => $_has(5);
  @$pb.TagNumber(6)
  void clearUpdated() => $_clearField(6);

  @$pb.TagNumber(7)
  $core.String get safetyCopy => $_getSZ(6);
  @$pb.TagNumber(7)
  set safetyCopy($core.String value) => $_setString(6, value);
  @$pb.TagNumber(7)
  $core.bool hasSafetyCopy() => $_has(6);
  @$pb.TagNumber(7)
  void clearSafetyCopy() => $_clearField(7);
}

class StageRestoreResponse extends $pb.GeneratedMessage {
  factory StageRestoreResponse({
    RestoreStatus? status,
  }) {
    final result = StageRestoreResponse._();
    if (status != null) result.status = status;
    return result;
  }

  StageRestoreResponse._();

  factory StageRestoreResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      StageRestoreResponse()..mergeFromBuffer(data, registry);
  factory StageRestoreResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      StageRestoreResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'StageRestoreResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: StageRestoreResponse.$_createMessage)
    ..aOM<RestoreStatus>(1, _omitFieldNames ? '' : 'status',
        subBuilder: RestoreStatus.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StageRestoreResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  StageRestoreResponse copyWith(void Function(StageRestoreResponse) updates) =>
      super.copyWith((message) => updates(message as StageRestoreResponse))
          as StageRestoreResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use StageRestoreResponse() / StageRestoreResponse.new instead')
  static StageRestoreResponse create() => StageRestoreResponse._();
  static $pb.GeneratedMessage $_createMessage() => StageRestoreResponse._();
  @$core.override
  StageRestoreResponse createEmptyInstance() => StageRestoreResponse._();
  @$core.pragma('dart2js:noInline')
  static StageRestoreResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<StageRestoreResponse>(
          StageRestoreResponse.$_createMessage);
  static StageRestoreResponse? _defaultInstance;

  @$pb.TagNumber(1)
  RestoreStatus get status => $_getN(0);
  @$pb.TagNumber(1)
  set status(RestoreStatus value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasStatus() => $_has(0);
  @$pb.TagNumber(1)
  void clearStatus() => $_clearField(1);
  @$pb.TagNumber(1)
  RestoreStatus ensureStatus() => $_ensure(0);
}

class GetRestoreStatusRequest extends $pb.GeneratedMessage {
  factory GetRestoreStatusRequest() => GetRestoreStatusRequest._();

  GetRestoreStatusRequest._();

  factory GetRestoreStatusRequest.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetRestoreStatusRequest()..mergeFromBuffer(data, registry);
  factory GetRestoreStatusRequest.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetRestoreStatusRequest()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetRestoreStatusRequest',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: GetRestoreStatusRequest.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetRestoreStatusRequest clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetRestoreStatusRequest copyWith(
          void Function(GetRestoreStatusRequest) updates) =>
      super.copyWith((message) => updates(message as GetRestoreStatusRequest))
          as GetRestoreStatusRequest;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use GetRestoreStatusRequest() / GetRestoreStatusRequest.new instead')
  static GetRestoreStatusRequest create() => GetRestoreStatusRequest._();
  static $pb.GeneratedMessage $_createMessage() => GetRestoreStatusRequest._();
  @$core.override
  GetRestoreStatusRequest createEmptyInstance() => GetRestoreStatusRequest._();
  @$core.pragma('dart2js:noInline')
  static GetRestoreStatusRequest getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetRestoreStatusRequest>(
          GetRestoreStatusRequest.$_createMessage);
  static GetRestoreStatusRequest? _defaultInstance;
}

class GetRestoreStatusResponse extends $pb.GeneratedMessage {
  factory GetRestoreStatusResponse({
    RestoreStatus? status,
  }) {
    final result = GetRestoreStatusResponse._();
    if (status != null) result.status = status;
    return result;
  }

  GetRestoreStatusResponse._();

  factory GetRestoreStatusResponse.fromBuffer($core.List<$core.int> data,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetRestoreStatusResponse()..mergeFromBuffer(data, registry);
  factory GetRestoreStatusResponse.fromJson($core.String json,
          [$pb.ExtensionRegistry registry = $pb.ExtensionRegistry.EMPTY]) =>
      GetRestoreStatusResponse()..mergeFromJson(json, registry);

  static final $pb.BuilderInfo _i = $pb.BuilderInfo(
      _omitMessageNames ? '' : 'GetRestoreStatusResponse',
      package: const $pb.PackageName(_omitMessageNames ? '' : 'itcmdr'),
      createEmptyInstance: GetRestoreStatusResponse.$_createMessage)
    ..aOM<RestoreStatus>(1, _omitFieldNames ? '' : 'status',
        subBuilder: RestoreStatus.$_createMessage)
    ..hasRequiredFields = false;

  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetRestoreStatusResponse clone() => deepCopy();
  @$core.Deprecated('See https://github.com/google/protobuf.dart/issues/998.')
  GetRestoreStatusResponse copyWith(
          void Function(GetRestoreStatusResponse) updates) =>
      super.copyWith((message) => updates(message as GetRestoreStatusResponse))
          as GetRestoreStatusResponse;

  @$core.override
  $pb.BuilderInfo get info_ => _i;

  @$core.pragma('dart2js:noInline')
  @$core.Deprecated(
      'Use GetRestoreStatusResponse() / GetRestoreStatusResponse.new instead')
  static GetRestoreStatusResponse create() => GetRestoreStatusResponse._();
  static $pb.GeneratedMessage $_createMessage() => GetRestoreStatusResponse._();
  @$core.override
  GetRestoreStatusResponse createEmptyInstance() =>
      GetRestoreStatusResponse._();
  @$core.pragma('dart2js:noInline')
  static GetRestoreStatusResponse getDefault() => _defaultInstance ??=
      $pb.GeneratedMessage.$_defaultFor<GetRestoreStatusResponse>(
          GetRestoreStatusResponse.$_createMessage);
  static GetRestoreStatusResponse? _defaultInstance;

  @$pb.TagNumber(1)
  RestoreStatus get status => $_getN(0);
  @$pb.TagNumber(1)
  set status(RestoreStatus value) => $_setField(1, value);
  @$pb.TagNumber(1)
  $core.bool hasStatus() => $_has(0);
  @$pb.TagNumber(1)
  void clearStatus() => $_clearField(1);
  @$pb.TagNumber(1)
  RestoreStatus ensureStatus() => $_ensure(0);
}

const $core.bool _omitFieldNames =
    $core.bool.fromEnvironment('protobuf.omit_field_names');
const $core.bool _omitMessageNames =
    $core.bool.fromEnvironment('protobuf.omit_message_names');
