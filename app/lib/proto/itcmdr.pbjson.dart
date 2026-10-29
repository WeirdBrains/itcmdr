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

@$core.Deprecated('Use rollupDescriptor instead')
const Rollup$json = {
  '1': 'Rollup',
  '2': [
    {'1': 'ROLLUP_DAILY', '2': 0},
    {'1': 'ROLLUP_WEEKLY', '2': 1},
    {'1': 'ROLLUP_MONTHLY', '2': 2},
    {'1': 'ROLLUP_QUARTERLY', '2': 3},
    {'1': 'ROLLUP_YEARLY', '2': 4},
  ],
};

/// Descriptor for `Rollup`. Decode as a `google.protobuf.EnumDescriptorProto`.
final $typed_data.Uint8List rollupDescriptor = $convert.base64Decode(
    'CgZSb2xsdXASEAoMUk9MTFVQX0RBSUxZEAASEQoNUk9MTFVQX1dFRUtMWRABEhIKDlJPTExVUF'
    '9NT05USExZEAISFAoQUk9MTFVQX1FVQVJURVJMWRADEhEKDVJPTExVUF9ZRUFSTFkQBA==');

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

@$core.Deprecated('Use getMetricsRequestDescriptor instead')
const GetMetricsRequest$json = {
  '1': 'GetMetricsRequest',
  '2': [
    {'1': 'window_days', '3': 1, '4': 1, '5': 5, '10': 'windowDays'},
    {
      '1': 'rollup',
      '3': 2,
      '4': 1,
      '5': 14,
      '6': '.itcmdr.Rollup',
      '10': 'rollup'
    },
  ],
};

/// Descriptor for `GetMetricsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getMetricsRequestDescriptor = $convert.base64Decode(
    'ChFHZXRNZXRyaWNzUmVxdWVzdBIfCgt3aW5kb3dfZGF5cxgBIAEoBVIKd2luZG93RGF5cxImCg'
    'Zyb2xsdXAYAiABKA4yDi5pdGNtZHIuUm9sbHVwUgZyb2xsdXA=');

@$core.Deprecated('Use metricPointDescriptor instead')
const MetricPoint$json = {
  '1': 'MetricPoint',
  '2': [
    {'1': 'key', '3': 1, '4': 1, '5': 9, '10': 'key'},
    {'1': 'ts', '3': 2, '4': 1, '5': 3, '10': 'ts'},
    {'1': 'value', '3': 3, '4': 1, '5': 1, '10': 'value'},
  ],
};

/// Descriptor for `MetricPoint`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List metricPointDescriptor = $convert.base64Decode(
    'CgtNZXRyaWNQb2ludBIQCgNrZXkYASABKAlSA2tleRIOCgJ0cxgCIAEoA1ICdHMSFAoFdmFsdW'
    'UYAyABKAFSBXZhbHVl');

@$core.Deprecated('Use getMetricsResponseDescriptor instead')
const GetMetricsResponse$json = {
  '1': 'GetMetricsResponse',
  '2': [
    {
      '1': 'points',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.MetricPoint',
      '10': 'points'
    },
  ],
};

/// Descriptor for `GetMetricsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getMetricsResponseDescriptor = $convert.base64Decode(
    'ChJHZXRNZXRyaWNzUmVzcG9uc2USKwoGcG9pbnRzGAEgAygLMhMuaXRjbWRyLk1ldHJpY1BvaW'
    '50UgZwb2ludHM=');

@$core.Deprecated('Use exceptionScopeDescriptor instead')
const ExceptionScope$json = {
  '1': 'ExceptionScope',
  '2': [
    {'1': 'machines', '3': 1, '4': 3, '5': 9, '10': 'machines'},
    {'1': 'role', '3': 2, '4': 1, '5': 9, '10': 'role'},
  ],
};

/// Descriptor for `ExceptionScope`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List exceptionScopeDescriptor = $convert.base64Decode(
    'Cg5FeGNlcHRpb25TY29wZRIaCghtYWNoaW5lcxgBIAMoCVIIbWFjaGluZXMSEgoEcm9sZRgCIA'
    'EoCVIEcm9sZQ==');

@$core.Deprecated('Use exceptionRuleDescriptor instead')
const ExceptionRule$json = {
  '1': 'ExceptionRule',
  '2': [
    {'1': 'id', '3': 1, '4': 1, '5': 9, '10': 'id'},
    {
      '1': 'scope',
      '3': 2,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.ExceptionScope',
      '10': 'scope'
    },
    {'1': 'check_id', '3': 3, '4': 1, '5': 9, '10': 'checkId'},
    {'1': 'check_pattern', '3': 4, '4': 1, '5': 9, '10': 'checkPattern'},
    {'1': 'action', '3': 5, '4': 1, '5': 9, '10': 'action'},
    {'1': 'reason', '3': 6, '4': 1, '5': 9, '10': 'reason'},
    {'1': 'expires', '3': 7, '4': 1, '5': 9, '10': 'expires'},
    {'1': 'active', '3': 8, '4': 1, '5': 8, '10': 'active'},
  ],
};

/// Descriptor for `ExceptionRule`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List exceptionRuleDescriptor = $convert.base64Decode(
    'Cg1FeGNlcHRpb25SdWxlEg4KAmlkGAEgASgJUgJpZBIsCgVzY29wZRgCIAEoCzIWLml0Y21kci'
    '5FeGNlcHRpb25TY29wZVIFc2NvcGUSGQoIY2hlY2tfaWQYAyABKAlSB2NoZWNrSWQSIwoNY2hl'
    'Y2tfcGF0dGVybhgEIAEoCVIMY2hlY2tQYXR0ZXJuEhYKBmFjdGlvbhgFIAEoCVIGYWN0aW9uEh'
    'YKBnJlYXNvbhgGIAEoCVIGcmVhc29uEhgKB2V4cGlyZXMYByABKAlSB2V4cGlyZXMSFgoGYWN0'
    'aXZlGAggASgIUgZhY3RpdmU=');

@$core.Deprecated('Use listExceptionsRequestDescriptor instead')
const ListExceptionsRequest$json = {
  '1': 'ListExceptionsRequest',
};

/// Descriptor for `ListExceptionsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listExceptionsRequestDescriptor =
    $convert.base64Decode('ChVMaXN0RXhjZXB0aW9uc1JlcXVlc3Q=');

@$core.Deprecated('Use listExceptionsResponseDescriptor instead')
const ListExceptionsResponse$json = {
  '1': 'ListExceptionsResponse',
  '2': [
    {
      '1': 'rules',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.ExceptionRule',
      '10': 'rules'
    },
  ],
};

/// Descriptor for `ListExceptionsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listExceptionsResponseDescriptor =
    $convert.base64Decode(
        'ChZMaXN0RXhjZXB0aW9uc1Jlc3BvbnNlEisKBXJ1bGVzGAEgAygLMhUuaXRjbWRyLkV4Y2VwdG'
        'lvblJ1bGVSBXJ1bGVz');

@$core.Deprecated('Use saveExceptionRequestDescriptor instead')
const SaveExceptionRequest$json = {
  '1': 'SaveExceptionRequest',
  '2': [
    {
      '1': 'rule',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.ExceptionRule',
      '10': 'rule'
    },
  ],
};

/// Descriptor for `SaveExceptionRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List saveExceptionRequestDescriptor = $convert.base64Decode(
    'ChRTYXZlRXhjZXB0aW9uUmVxdWVzdBIpCgRydWxlGAEgASgLMhUuaXRjbWRyLkV4Y2VwdGlvbl'
    'J1bGVSBHJ1bGU=');

@$core.Deprecated('Use saveExceptionResponseDescriptor instead')
const SaveExceptionResponse$json = {
  '1': 'SaveExceptionResponse',
  '2': [
    {
      '1': 'rule',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.ExceptionRule',
      '10': 'rule'
    },
  ],
};

/// Descriptor for `SaveExceptionResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List saveExceptionResponseDescriptor = $convert.base64Decode(
    'ChVTYXZlRXhjZXB0aW9uUmVzcG9uc2USKQoEcnVsZRgBIAEoCzIVLml0Y21kci5FeGNlcHRpb2'
    '5SdWxlUgRydWxl');

@$core.Deprecated('Use expireExceptionRequestDescriptor instead')
const ExpireExceptionRequest$json = {
  '1': 'ExpireExceptionRequest',
  '2': [
    {'1': 'id', '3': 1, '4': 1, '5': 9, '10': 'id'},
  ],
};

/// Descriptor for `ExpireExceptionRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List expireExceptionRequestDescriptor = $convert
    .base64Decode('ChZFeHBpcmVFeGNlcHRpb25SZXF1ZXN0Eg4KAmlkGAEgASgJUgJpZA==');

@$core.Deprecated('Use expireExceptionResponseDescriptor instead')
const ExpireExceptionResponse$json = {
  '1': 'ExpireExceptionResponse',
};

/// Descriptor for `ExpireExceptionResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List expireExceptionResponseDescriptor =
    $convert.base64Decode('ChdFeHBpcmVFeGNlcHRpb25SZXNwb25zZQ==');

@$core.Deprecated('Use searchLogsRequestDescriptor instead')
const SearchLogsRequest$json = {
  '1': 'SearchLogsRequest',
  '2': [
    {'1': 'query', '3': 1, '4': 1, '5': 9, '10': 'query'},
    {'1': 'machine', '3': 2, '4': 1, '5': 9, '10': 'machine'},
    {'1': 'start_ts', '3': 3, '4': 1, '5': 3, '10': 'startTs'},
    {'1': 'end_ts', '3': 4, '4': 1, '5': 3, '10': 'endTs'},
    {'1': 'limit', '3': 5, '4': 1, '5': 5, '10': 'limit'},
  ],
};

/// Descriptor for `SearchLogsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List searchLogsRequestDescriptor = $convert.base64Decode(
    'ChFTZWFyY2hMb2dzUmVxdWVzdBIUCgVxdWVyeRgBIAEoCVIFcXVlcnkSGAoHbWFjaGluZRgCIA'
    'EoCVIHbWFjaGluZRIZCghzdGFydF90cxgDIAEoA1IHc3RhcnRUcxIVCgZlbmRfdHMYBCABKANS'
    'BWVuZFRzEhQKBWxpbWl0GAUgASgFUgVsaW1pdA==');

@$core.Deprecated('Use searchLogsResponseDescriptor instead')
const SearchLogsResponse$json = {
  '1': 'SearchLogsResponse',
  '2': [
    {'1': 'lines', '3': 1, '4': 3, '5': 9, '10': 'lines'},
  ],
};

/// Descriptor for `SearchLogsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List searchLogsResponseDescriptor = $convert
    .base64Decode('ChJTZWFyY2hMb2dzUmVzcG9uc2USFAoFbGluZXMYASADKAlSBWxpbmVz');

@$core.Deprecated('Use watchUpdatesRequestDescriptor instead')
const WatchUpdatesRequest$json = {
  '1': 'WatchUpdatesRequest',
};

/// Descriptor for `WatchUpdatesRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List watchUpdatesRequestDescriptor =
    $convert.base64Decode('ChNXYXRjaFVwZGF0ZXNSZXF1ZXN0');

@$core.Deprecated('Use updateDescriptor instead')
const Update$json = {
  '1': 'Update',
  '2': [
    {'1': 'kind', '3': 1, '4': 1, '5': 9, '10': 'kind'},
    {'1': 'entity', '3': 2, '4': 1, '5': 9, '10': 'entity'},
    {'1': 'id', '3': 3, '4': 1, '5': 9, '10': 'id'},
    {'1': 'ts', '3': 4, '4': 1, '5': 3, '10': 'ts'},
  ],
};

/// Descriptor for `Update`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List updateDescriptor = $convert.base64Decode(
    'CgZVcGRhdGUSEgoEa2luZBgBIAEoCVIEa2luZBIWCgZlbnRpdHkYAiABKAlSBmVudGl0eRIOCg'
    'JpZBgDIAEoCVICaWQSDgoCdHMYBCABKANSAnRz');
