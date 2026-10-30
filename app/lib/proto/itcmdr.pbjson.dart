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
    {'1': 'ROLLUP_BLOCK4', '2': 2},
    {'1': 'ROLLUP_YEARLY', '2': 4},
  ],
  '4': [
    {'1': 3, '2': 3},
  ],
};

/// Descriptor for `Rollup`. Decode as a `google.protobuf.EnumDescriptorProto`.
final $typed_data.Uint8List rollupDescriptor = $convert.base64Decode(
    'CgZSb2xsdXASEAoMUk9MTFVQX0RBSUxZEAASEQoNUk9MTFVQX1dFRUtMWRABEhEKDVJPTExVUF'
    '9CTE9DSzQQAhIRCg1ST0xMVVBfWUVBUkxZEAQiBAgDEAM=');

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
    {'1': 'TIMED_OUT', '2': 5},
  ],
};

/// Descriptor for `RequestStatus`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List requestStatusDescriptor = $convert.base64Decode(
    'Cg1SZXF1ZXN0U3RhdHVzEjQKBnN0YXR1cxgBIAEoDjIcLml0Y21kci5SZXF1ZXN0U3RhdHVzLl'
    'N0YXR1c1IGc3RhdHVzEhgKB21lc3NhZ2UYAiABKAlSB21lc3NhZ2UiUwoGU3RhdHVzEgsKB1VO'
    'S05PV04QABIKCgZRVUVVRUQQARILCgdSVU5OSU5HEAISCAoERE9ORRADEgoKBkZBSUxFRBAEEg'
    '0KCVRJTUVEX09VVBAF');

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
  '2': [
    {'1': 'include_inactive', '3': 1, '4': 1, '5': 8, '10': 'includeInactive'},
  ],
};

/// Descriptor for `ListExceptionsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listExceptionsRequestDescriptor = $convert.base64Decode(
    'ChVMaXN0RXhjZXB0aW9uc1JlcXVlc3QSKQoQaW5jbHVkZV9pbmFjdGl2ZRgBIAEoCFIPaW5jbH'
    'VkZUluYWN0aXZl');

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
    {'1': 'category', '3': 6, '4': 1, '5': 9, '10': 'category'},
    {'1': 'status', '3': 7, '4': 1, '5': 9, '10': 'status'},
    {'1': 'offset', '3': 8, '4': 1, '5': 5, '10': 'offset'},
  ],
};

/// Descriptor for `SearchLogsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List searchLogsRequestDescriptor = $convert.base64Decode(
    'ChFTZWFyY2hMb2dzUmVxdWVzdBIUCgVxdWVyeRgBIAEoCVIFcXVlcnkSGAoHbWFjaGluZRgCIA'
    'EoCVIHbWFjaGluZRIZCghzdGFydF90cxgDIAEoA1IHc3RhcnRUcxIVCgZlbmRfdHMYBCABKANS'
    'BWVuZFRzEhQKBWxpbWl0GAUgASgFUgVsaW1pdBIaCghjYXRlZ29yeRgGIAEoCVIIY2F0ZWdvcn'
    'kSFgoGc3RhdHVzGAcgASgJUgZzdGF0dXMSFgoGb2Zmc2V0GAggASgFUgZvZmZzZXQ=');

@$core.Deprecated('Use searchLogsResponseDescriptor instead')
const SearchLogsResponse$json = {
  '1': 'SearchLogsResponse',
  '2': [
    {'1': 'lines', '3': 1, '4': 3, '5': 9, '10': 'lines'},
    {'1': 'total', '3': 2, '4': 1, '5': 5, '10': 'total'},
    {'1': 'next_offset', '3': 3, '4': 1, '5': 5, '10': 'nextOffset'},
  ],
};

/// Descriptor for `SearchLogsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List searchLogsResponseDescriptor = $convert.base64Decode(
    'ChJTZWFyY2hMb2dzUmVzcG9uc2USFAoFbGluZXMYASADKAlSBWxpbmVzEhQKBXRvdGFsGAIgAS'
    'gFUgV0b3RhbBIfCgtuZXh0X29mZnNldBgDIAEoBVIKbmV4dE9mZnNldA==');

@$core.Deprecated('Use auditEntryDescriptor instead')
const AuditEntry$json = {
  '1': 'AuditEntry',
  '2': [
    {'1': 'ts', '3': 1, '4': 1, '5': 3, '10': 'ts'},
    {'1': 'actor', '3': 2, '4': 1, '5': 9, '10': 'actor'},
    {'1': 'action', '3': 3, '4': 1, '5': 9, '10': 'action'},
    {'1': 'detail', '3': 4, '4': 1, '5': 9, '10': 'detail'},
  ],
};

/// Descriptor for `AuditEntry`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List auditEntryDescriptor = $convert.base64Decode(
    'CgpBdWRpdEVudHJ5Eg4KAnRzGAEgASgDUgJ0cxIUCgVhY3RvchgCIAEoCVIFYWN0b3ISFgoGYW'
    'N0aW9uGAMgASgJUgZhY3Rpb24SFgoGZGV0YWlsGAQgASgJUgZkZXRhaWw=');

@$core.Deprecated('Use listAuditRequestDescriptor instead')
const ListAuditRequest$json = {
  '1': 'ListAuditRequest',
  '2': [
    {'1': 'limit', '3': 1, '4': 1, '5': 5, '10': 'limit'},
  ],
};

/// Descriptor for `ListAuditRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listAuditRequestDescriptor = $convert
    .base64Decode('ChBMaXN0QXVkaXRSZXF1ZXN0EhQKBWxpbWl0GAEgASgFUgVsaW1pdA==');

@$core.Deprecated('Use listAuditResponseDescriptor instead')
const ListAuditResponse$json = {
  '1': 'ListAuditResponse',
  '2': [
    {
      '1': 'entries',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.AuditEntry',
      '10': 'entries'
    },
  ],
};

/// Descriptor for `ListAuditResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listAuditResponseDescriptor = $convert.base64Decode(
    'ChFMaXN0QXVkaXRSZXNwb25zZRIsCgdlbnRyaWVzGAEgAygLMhIuaXRjbWRyLkF1ZGl0RW50cn'
    'lSB2VudHJpZXM=');

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

@$core.Deprecated('Use backupInfoDescriptor instead')
const BackupInfo$json = {
  '1': 'BackupInfo',
  '2': [
    {'1': 'name', '3': 1, '4': 1, '5': 9, '10': 'name'},
    {'1': 'size', '3': 2, '4': 1, '5': 3, '10': 'size'},
    {'1': 'created_ts', '3': 3, '4': 1, '5': 3, '10': 'createdTs'},
    {'1': 'kind', '3': 4, '4': 1, '5': 9, '10': 'kind'},
    {'1': 'itcmdr_version', '3': 5, '4': 1, '5': 9, '10': 'itcmdrVersion'},
    {'1': 'schema_version', '3': 6, '4': 1, '5': 3, '10': 'schemaVersion'},
  ],
};

/// Descriptor for `BackupInfo`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List backupInfoDescriptor = $convert.base64Decode(
    'CgpCYWNrdXBJbmZvEhIKBG5hbWUYASABKAlSBG5hbWUSEgoEc2l6ZRgCIAEoA1IEc2l6ZRIdCg'
    'pjcmVhdGVkX3RzGAMgASgDUgljcmVhdGVkVHMSEgoEa2luZBgEIAEoCVIEa2luZBIlCg5pdGNt'
    'ZHJfdmVyc2lvbhgFIAEoCVINaXRjbWRyVmVyc2lvbhIlCg5zY2hlbWFfdmVyc2lvbhgGIAEoA1'
    'INc2NoZW1hVmVyc2lvbg==');

@$core.Deprecated('Use createBackupRequestDescriptor instead')
const CreateBackupRequest$json = {
  '1': 'CreateBackupRequest',
};

/// Descriptor for `CreateBackupRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List createBackupRequestDescriptor =
    $convert.base64Decode('ChNDcmVhdGVCYWNrdXBSZXF1ZXN0');

@$core.Deprecated('Use createBackupResponseDescriptor instead')
const CreateBackupResponse$json = {
  '1': 'CreateBackupResponse',
  '2': [
    {
      '1': 'backup',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.BackupInfo',
      '10': 'backup'
    },
  ],
};

/// Descriptor for `CreateBackupResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List createBackupResponseDescriptor = $convert.base64Decode(
    'ChRDcmVhdGVCYWNrdXBSZXNwb25zZRIqCgZiYWNrdXAYASABKAsyEi5pdGNtZHIuQmFja3VwSW'
    '5mb1IGYmFja3Vw');

@$core.Deprecated('Use listBackupsRequestDescriptor instead')
const ListBackupsRequest$json = {
  '1': 'ListBackupsRequest',
};

/// Descriptor for `ListBackupsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listBackupsRequestDescriptor =
    $convert.base64Decode('ChJMaXN0QmFja3Vwc1JlcXVlc3Q=');

@$core.Deprecated('Use listBackupsResponseDescriptor instead')
const ListBackupsResponse$json = {
  '1': 'ListBackupsResponse',
  '2': [
    {
      '1': 'backups',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.BackupInfo',
      '10': 'backups'
    },
  ],
};

/// Descriptor for `ListBackupsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listBackupsResponseDescriptor = $convert.base64Decode(
    'ChNMaXN0QmFja3Vwc1Jlc3BvbnNlEiwKB2JhY2t1cHMYASADKAsyEi5pdGNtZHIuQmFja3VwSW'
    '5mb1IHYmFja3Vwcw==');

@$core.Deprecated('Use verifyBackupRequestDescriptor instead')
const VerifyBackupRequest$json = {
  '1': 'VerifyBackupRequest',
  '2': [
    {'1': 'name', '3': 1, '4': 1, '5': 9, '10': 'name'},
  ],
};

/// Descriptor for `VerifyBackupRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List verifyBackupRequestDescriptor = $convert
    .base64Decode('ChNWZXJpZnlCYWNrdXBSZXF1ZXN0EhIKBG5hbWUYASABKAlSBG5hbWU=');

@$core.Deprecated('Use verifyBackupResponseDescriptor instead')
const VerifyBackupResponse$json = {
  '1': 'VerifyBackupResponse',
  '2': [
    {'1': 'ok', '3': 1, '4': 1, '5': 8, '10': 'ok'},
    {'1': 'message', '3': 2, '4': 1, '5': 9, '10': 'message'},
  ],
};

/// Descriptor for `VerifyBackupResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List verifyBackupResponseDescriptor = $convert.base64Decode(
    'ChRWZXJpZnlCYWNrdXBSZXNwb25zZRIOCgJvaxgBIAEoCFICb2sSGAoHbWVzc2FnZRgCIAEoCV'
    'IHbWVzc2FnZQ==');

@$core.Deprecated('Use backupSettingsDescriptor instead')
const BackupSettings$json = {
  '1': 'BackupSettings',
  '2': [
    {'1': 'dir', '3': 1, '4': 1, '5': 9, '10': 'dir'},
    {'1': 'nightly', '3': 2, '4': 1, '5': 8, '10': 'nightly'},
    {'1': 'hour', '3': 3, '4': 1, '5': 13, '10': 'hour'},
    {'1': 'keep_daily', '3': 4, '4': 1, '5': 13, '10': 'keepDaily'},
    {'1': 'keep_weekly', '3': 5, '4': 1, '5': 13, '10': 'keepWeekly'},
    {'1': 'keep_blocks', '3': 6, '4': 1, '5': 13, '10': 'keepBlocks'},
  ],
};

/// Descriptor for `BackupSettings`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List backupSettingsDescriptor = $convert.base64Decode(
    'Cg5CYWNrdXBTZXR0aW5ncxIQCgNkaXIYASABKAlSA2RpchIYCgduaWdodGx5GAIgASgIUgduaW'
    'dodGx5EhIKBGhvdXIYAyABKA1SBGhvdXISHQoKa2VlcF9kYWlseRgEIAEoDVIJa2VlcERhaWx5'
    'Eh8KC2tlZXBfd2Vla2x5GAUgASgNUgprZWVwV2Vla2x5Eh8KC2tlZXBfYmxvY2tzGAYgASgNUg'
    'prZWVwQmxvY2tz');

@$core.Deprecated('Use getSettingsRequestDescriptor instead')
const GetSettingsRequest$json = {
  '1': 'GetSettingsRequest',
};

/// Descriptor for `GetSettingsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getSettingsRequestDescriptor =
    $convert.base64Decode('ChJHZXRTZXR0aW5nc1JlcXVlc3Q=');

@$core.Deprecated('Use getSettingsResponseDescriptor instead')
const GetSettingsResponse$json = {
  '1': 'GetSettingsResponse',
  '2': [
    {
      '1': 'backup',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.BackupSettings',
      '10': 'backup'
    },
  ],
};

/// Descriptor for `GetSettingsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getSettingsResponseDescriptor = $convert.base64Decode(
    'ChNHZXRTZXR0aW5nc1Jlc3BvbnNlEi4KBmJhY2t1cBgBIAEoCzIWLml0Y21kci5CYWNrdXBTZX'
    'R0aW5nc1IGYmFja3Vw');

@$core.Deprecated('Use updateSettingsRequestDescriptor instead')
const UpdateSettingsRequest$json = {
  '1': 'UpdateSettingsRequest',
  '2': [
    {
      '1': 'backup',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.BackupSettings',
      '10': 'backup'
    },
  ],
};

/// Descriptor for `UpdateSettingsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List updateSettingsRequestDescriptor = $convert.base64Decode(
    'ChVVcGRhdGVTZXR0aW5nc1JlcXVlc3QSLgoGYmFja3VwGAEgASgLMhYuaXRjbWRyLkJhY2t1cF'
    'NldHRpbmdzUgZiYWNrdXA=');

@$core.Deprecated('Use updateSettingsResponseDescriptor instead')
const UpdateSettingsResponse$json = {
  '1': 'UpdateSettingsResponse',
  '2': [
    {
      '1': 'backup',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.BackupSettings',
      '10': 'backup'
    },
  ],
};

/// Descriptor for `UpdateSettingsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List updateSettingsResponseDescriptor =
    $convert.base64Decode(
        'ChZVcGRhdGVTZXR0aW5nc1Jlc3BvbnNlEi4KBmJhY2t1cBgBIAEoCzIWLml0Y21kci5CYWNrdX'
        'BTZXR0aW5nc1IGYmFja3Vw');

@$core.Deprecated('Use settingItemDescriptor instead')
const SettingItem$json = {
  '1': 'SettingItem',
  '2': [
    {'1': 'key', '3': 1, '4': 1, '5': 9, '10': 'key'},
    {'1': 'scope', '3': 2, '4': 1, '5': 9, '10': 'scope'},
    {'1': 'type', '3': 3, '4': 1, '5': 9, '10': 'type'},
    {'1': 'value', '3': 4, '4': 1, '5': 9, '10': 'value'},
    {'1': 'default_value', '3': 5, '4': 1, '5': 9, '10': 'defaultValue'},
    {'1': 'source', '3': 6, '4': 1, '5': 9, '10': 'source'},
    {'1': 'description', '3': 7, '4': 1, '5': 9, '10': 'description'},
    {'1': 'min', '3': 8, '4': 1, '5': 3, '10': 'min'},
    {'1': 'max', '3': 9, '4': 1, '5': 3, '10': 'max'},
    {'1': 'read_only', '3': 10, '4': 1, '5': 8, '10': 'readOnly'},
    {'1': 'applies', '3': 11, '4': 1, '5': 9, '10': 'applies'},
  ],
};

/// Descriptor for `SettingItem`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List settingItemDescriptor = $convert.base64Decode(
    'CgtTZXR0aW5nSXRlbRIQCgNrZXkYASABKAlSA2tleRIUCgVzY29wZRgCIAEoCVIFc2NvcGUSEg'
    'oEdHlwZRgDIAEoCVIEdHlwZRIUCgV2YWx1ZRgEIAEoCVIFdmFsdWUSIwoNZGVmYXVsdF92YWx1'
    'ZRgFIAEoCVIMZGVmYXVsdFZhbHVlEhYKBnNvdXJjZRgGIAEoCVIGc291cmNlEiAKC2Rlc2NyaX'
    'B0aW9uGAcgASgJUgtkZXNjcmlwdGlvbhIQCgNtaW4YCCABKANSA21pbhIQCgNtYXgYCSABKANS'
    'A21heBIbCglyZWFkX29ubHkYCiABKAhSCHJlYWRPbmx5EhgKB2FwcGxpZXMYCyABKAlSB2FwcG'
    'xpZXM=');

@$core.Deprecated('Use listSettingsRequestDescriptor instead')
const ListSettingsRequest$json = {
  '1': 'ListSettingsRequest',
  '2': [
    {'1': 'machine', '3': 1, '4': 1, '5': 9, '10': 'machine'},
  ],
};

/// Descriptor for `ListSettingsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listSettingsRequestDescriptor =
    $convert.base64Decode(
        'ChNMaXN0U2V0dGluZ3NSZXF1ZXN0EhgKB21hY2hpbmUYASABKAlSB21hY2hpbmU=');

@$core.Deprecated('Use listSettingsResponseDescriptor instead')
const ListSettingsResponse$json = {
  '1': 'ListSettingsResponse',
  '2': [
    {
      '1': 'settings',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.SettingItem',
      '10': 'settings'
    },
    {
      '1': 'machines_with_overrides',
      '3': 2,
      '4': 3,
      '5': 9,
      '10': 'machinesWithOverrides'
    },
  ],
};

/// Descriptor for `ListSettingsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listSettingsResponseDescriptor = $convert.base64Decode(
    'ChRMaXN0U2V0dGluZ3NSZXNwb25zZRIvCghzZXR0aW5ncxgBIAMoCzITLml0Y21kci5TZXR0aW'
    '5nSXRlbVIIc2V0dGluZ3MSNgoXbWFjaGluZXNfd2l0aF9vdmVycmlkZXMYAiADKAlSFW1hY2hp'
    'bmVzV2l0aE92ZXJyaWRlcw==');

@$core.Deprecated('Use updateSettingRequestDescriptor instead')
const UpdateSettingRequest$json = {
  '1': 'UpdateSettingRequest',
  '2': [
    {'1': 'key', '3': 1, '4': 1, '5': 9, '10': 'key'},
    {'1': 'value', '3': 2, '4': 1, '5': 9, '10': 'value'},
    {'1': 'machine', '3': 3, '4': 1, '5': 9, '10': 'machine'},
    {'1': 'unset', '3': 4, '4': 1, '5': 8, '10': 'unset'},
  ],
};

/// Descriptor for `UpdateSettingRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List updateSettingRequestDescriptor = $convert.base64Decode(
    'ChRVcGRhdGVTZXR0aW5nUmVxdWVzdBIQCgNrZXkYASABKAlSA2tleRIUCgV2YWx1ZRgCIAEoCV'
    'IFdmFsdWUSGAoHbWFjaGluZRgDIAEoCVIHbWFjaGluZRIUCgV1bnNldBgEIAEoCFIFdW5zZXQ=');

@$core.Deprecated('Use updateSettingResponseDescriptor instead')
const UpdateSettingResponse$json = {
  '1': 'UpdateSettingResponse',
  '2': [
    {
      '1': 'setting',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.SettingItem',
      '10': 'setting'
    },
  ],
};

/// Descriptor for `UpdateSettingResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List updateSettingResponseDescriptor = $convert.base64Decode(
    'ChVVcGRhdGVTZXR0aW5nUmVzcG9uc2USLQoHc2V0dGluZxgBIAEoCzITLml0Y21kci5TZXR0aW'
    '5nSXRlbVIHc2V0dGluZw==');
