// This is a generated file - do not edit.
//
// Generated from itcmdr.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports
// ignore_for_file: unused_import

import 'dart:convert' as $convert;
import 'dart:core' as $core;
import 'dart:typed_data' as $typed_data;

@$core.Deprecated('Use machineDescriptor instead')
const Machine$json = {
  '1': 'Machine',
  '2': [
    {'1': 'name', '3': 1, '4': 1, '5': 9, '10': 'name'},
    {'1': 'status', '3': 2, '4': 1, '5': 9, '10': 'status'},
    {'1': 'open_incidents', '3': 3, '4': 1, '5': 5, '10': 'openIncidents'},
    {'1': 'last_run_ts', '3': 4, '4': 1, '5': 3, '10': 'lastRunTs'},
  ],
};

/// Descriptor for `Machine`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List machineDescriptor = $convert.base64Decode(
    'CgdNYWNoaW5lEhIKBG5hbWUYASABKAlSBG5hbWUSFgoGc3RhdHVzGAIgASgJUgZzdGF0dXMSJQ'
    'oOb3Blbl9pbmNpZGVudHMYAyABKAVSDW9wZW5JbmNpZGVudHMSHgoLbGFzdF9ydW5fdHMYBCAB'
    'KANSCWxhc3RSdW5Ucw==');

@$core.Deprecated('Use listMachinesRequestDescriptor instead')
const ListMachinesRequest$json = {
  '1': 'ListMachinesRequest',
};

/// Descriptor for `ListMachinesRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listMachinesRequestDescriptor =
    $convert.base64Decode('ChNMaXN0TWFjaGluZXNSZXF1ZXN0');

@$core.Deprecated('Use listMachinesResponseDescriptor instead')
const ListMachinesResponse$json = {
  '1': 'ListMachinesResponse',
  '2': [
    {
      '1': 'machines',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.Machine',
      '10': 'machines'
    },
  ],
};

/// Descriptor for `ListMachinesResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listMachinesResponseDescriptor = $convert.base64Decode(
    'ChRMaXN0TWFjaGluZXNSZXNwb25zZRIrCghtYWNoaW5lcxgBIAMoCzIPLml0Y21kci5NYWNoaW'
    '5lUghtYWNoaW5lcw==');

@$core.Deprecated('Use triggerRunsRequestDescriptor instead')
const TriggerRunsRequest$json = {
  '1': 'TriggerRunsRequest',
  '2': [
    {'1': 'machines', '3': 1, '4': 3, '5': 9, '10': 'machines'},
    {'1': 'report_only', '3': 2, '4': 1, '5': 8, '10': 'reportOnly'},
    {'1': 'repair', '3': 3, '4': 1, '5': 8, '10': 'repair'},
    {'1': 'force', '3': 4, '4': 1, '5': 8, '10': 'force'},
    {'1': 'categories', '3': 5, '4': 3, '5': 9, '10': 'categories'},
  ],
};

/// Descriptor for `TriggerRunsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List triggerRunsRequestDescriptor = $convert.base64Decode(
    'ChJUcmlnZ2VyUnVuc1JlcXVlc3QSGgoIbWFjaGluZXMYASADKAlSCG1hY2hpbmVzEh8KC3JlcG'
    '9ydF9vbmx5GAIgASgIUgpyZXBvcnRPbmx5EhYKBnJlcGFpchgDIAEoCFIGcmVwYWlyEhQKBWZv'
    'cmNlGAQgASgIUgVmb3JjZRIeCgpjYXRlZ29yaWVzGAUgAygJUgpjYXRlZ29yaWVz');

@$core.Deprecated('Use triggerRunsResponseDescriptor instead')
const TriggerRunsResponse$json = {
  '1': 'TriggerRunsResponse',
  '2': [
    {'1': 'request_id', '3': 1, '4': 1, '5': 9, '10': 'requestId'},
  ],
};

/// Descriptor for `TriggerRunsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List triggerRunsResponseDescriptor = $convert.base64Decode(
    'ChNUcmlnZ2VyUnVuc1Jlc3BvbnNlEh0KCnJlcXVlc3RfaWQYASABKAlSCXJlcXVlc3RJZA==');

@$core.Deprecated('Use getRequestStatusRequestDescriptor instead')
const GetRequestStatusRequest$json = {
  '1': 'GetRequestStatusRequest',
  '2': [
    {'1': 'request_id', '3': 1, '4': 1, '5': 9, '10': 'requestId'},
  ],
};

/// Descriptor for `GetRequestStatusRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getRequestStatusRequestDescriptor =
    $convert.base64Decode(
        'ChdHZXRSZXF1ZXN0U3RhdHVzUmVxdWVzdBIdCgpyZXF1ZXN0X2lkGAEgASgJUglyZXF1ZXN0SW'
        'Q=');

@$core.Deprecated('Use requestStatusDescriptor instead')
const RequestStatus$json = {
  '1': 'RequestStatus',
  '2': [
    {
      '1': 'status',
      '3': 1,
      '4': 1,
      '5': 14,
      '6': '.itcmdr.RequestStatus.Status',
      '10': 'status'
    },
    {'1': 'message', '3': 2, '4': 1, '5': 9, '10': 'message'},
  ],
  '4': [RequestStatus_Status$json],
};

@$core.Deprecated('Use requestStatusDescriptor instead')
const RequestStatus_Status$json = {
  '1': 'Status',
  '2': [
    {'1': 'UNKNOWN', '2': 0},
    {'1': 'QUEUED', '2': 1},
    {'1': 'RUNNING', '2': 2},
    {'1': 'DONE', '2': 3},
    {'1': 'FAILED', '2': 4},
  ],
};

/// Descriptor for `RequestStatus`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List requestStatusDescriptor = $convert.base64Decode(
    'Cg1SZXF1ZXN0U3RhdHVzEjQKBnN0YXR1cxgBIAEoDjIcLml0Y21kci5SZXF1ZXN0U3RhdHVzLl'
    'N0YXR1c1IGc3RhdHVzEhgKB21lc3NhZ2UYAiABKAlSB21lc3NhZ2UiRAoGU3RhdHVzEgsKB1VO'
    'S05PV04QABIKCgZRVUVVRUQQARILCgdSVU5OSU5HEAISCAoERE9ORRADEgoKBkZBSUxFRBAE');

@$core.Deprecated('Use getRequestStatusResponseDescriptor instead')
const GetRequestStatusResponse$json = {
  '1': 'GetRequestStatusResponse',
  '2': [
    {
      '1': 'status',
      '3': 1,
      '4': 1,
      '5': 14,
      '6': '.itcmdr.RequestStatus.Status',
      '10': 'status'
    },
    {'1': 'message', '3': 2, '4': 1, '5': 9, '10': 'message'},
  ],
};

/// Descriptor for `GetRequestStatusResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getRequestStatusResponseDescriptor =
    $convert.base64Decode(
        'ChhHZXRSZXF1ZXN0U3RhdHVzUmVzcG9uc2USNAoGc3RhdHVzGAEgASgOMhwuaXRjbWRyLlJlcX'
        'Vlc3RTdGF0dXMuU3RhdHVzUgZzdGF0dXMSGAoHbWVzc2FnZRgCIAEoCVIHbWVzc2FnZQ==');
