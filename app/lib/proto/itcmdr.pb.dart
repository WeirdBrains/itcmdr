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

const $core.bool _omitFieldNames =
    $core.bool.fromEnvironment('protobuf.omit_field_names');
const $core.bool _omitMessageNames =
    $core.bool.fromEnvironment('protobuf.omit_message_names');
