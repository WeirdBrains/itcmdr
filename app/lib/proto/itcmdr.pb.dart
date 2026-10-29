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
  }) {
    final result = TriggerRunsRequest._();
    if (machines != null) result.machines.addAll(machines);
    if (reportOnly != null) result.reportOnly = reportOnly;
    if (repair != null) result.repair = repair;
    if (force != null) result.force = force;
    if (categories != null) result.categories.addAll(categories);
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
  factory ListExceptionsRequest() => ListExceptionsRequest._();

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

const $core.bool _omitFieldNames =
    $core.bool.fromEnvironment('protobuf.omit_field_names');
const $core.bool _omitMessageNames =
    $core.bool.fromEnvironment('protobuf.omit_message_names');
