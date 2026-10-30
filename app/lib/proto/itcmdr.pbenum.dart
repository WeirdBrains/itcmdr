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

import 'package:protobuf/protobuf.dart' as $pb;

/// Metrics and rollups
/// Buckets are aligned to ISO weeks: weekly = ISO week (Mon-Sun);
/// block4 = 4-ISO-week block N covering weeks 4N-3..4N (13 per ISO year,
/// week 53 folds into block 13); yearly = ISO year.
class Rollup extends $pb.ProtobufEnum {
  static const Rollup ROLLUP_DAILY =
      Rollup._(0, _omitEnumNames ? '' : 'ROLLUP_DAILY');
  static const Rollup ROLLUP_WEEKLY =
      Rollup._(1, _omitEnumNames ? '' : 'ROLLUP_WEEKLY');
  static const Rollup ROLLUP_BLOCK4 =
      Rollup._(2, _omitEnumNames ? '' : 'ROLLUP_BLOCK4');
  static const Rollup ROLLUP_YEARLY =
      Rollup._(4, _omitEnumNames ? '' : 'ROLLUP_YEARLY');

  static const $core.List<Rollup> values = <Rollup>[
    ROLLUP_DAILY,
    ROLLUP_WEEKLY,
    ROLLUP_BLOCK4,
    ROLLUP_YEARLY,
  ];

  static final $core.List<Rollup?> _byValue =
      $pb.ProtobufEnum.$_initByValueList(values, 4);
  static Rollup? valueOf($core.int value) =>
      value < 0 || value >= _byValue.length ? null : _byValue[value];

  const Rollup._(super.value, super.name);
}

class RequestStatus_Status extends $pb.ProtobufEnum {
  static const RequestStatus_Status UNKNOWN =
      RequestStatus_Status._(0, _omitEnumNames ? '' : 'UNKNOWN');
  static const RequestStatus_Status QUEUED =
      RequestStatus_Status._(1, _omitEnumNames ? '' : 'QUEUED');
  static const RequestStatus_Status RUNNING =
      RequestStatus_Status._(2, _omitEnumNames ? '' : 'RUNNING');
  static const RequestStatus_Status DONE =
      RequestStatus_Status._(3, _omitEnumNames ? '' : 'DONE');
  static const RequestStatus_Status FAILED =
      RequestStatus_Status._(4, _omitEnumNames ? '' : 'FAILED');
  static const RequestStatus_Status TIMED_OUT =
      RequestStatus_Status._(5, _omitEnumNames ? '' : 'TIMED_OUT');

  static const $core.List<RequestStatus_Status> values = <RequestStatus_Status>[
    UNKNOWN,
    QUEUED,
    RUNNING,
    DONE,
    FAILED,
    TIMED_OUT,
  ];

  static final $core.List<RequestStatus_Status?> _byValue =
      $pb.ProtobufEnum.$_initByValueList(values, 5);
  static RequestStatus_Status? valueOf($core.int value) =>
      value < 0 || value >= _byValue.length ? null : _byValue[value];

  const RequestStatus_Status._(super.value, super.name);
}

const $core.bool _omitEnumNames =
    $core.bool.fromEnvironment('protobuf.omit_enum_names');
