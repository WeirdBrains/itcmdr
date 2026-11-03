// Web: the browser validates HTTPS itself (its own warnings and exceptions apply).
import 'package:grpc/grpc_or_grpcweb.dart';

import 'cert_trust.dart';

GrpcOrGrpcWebClientChannel secureChannel(
  String host,
  int port,
  bool tls,
  TrustStore store,
) => GrpcOrGrpcWebClientChannel.toSingleEndpoint(
  host: host,
  port: port,
  transportSecure: tls,
);
