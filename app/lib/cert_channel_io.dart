// Native: HTTPS validated by the system trust store; a refused certificate is accepted only when
// it is exactly the one the user trusted for this server.
import 'dart:io';

import 'package:grpc/grpc.dart';
import 'package:grpc/grpc_or_grpcweb.dart';

import 'cert_trust.dart';

GrpcOrGrpcWebClientChannel secureChannel(
  String host,
  int port,
  bool tls,
  TrustStore store,
) {
  if (!tls) {
    return GrpcOrGrpcWebClientChannel.grpc(
      host,
      port: port,
      options: const ChannelOptions(credentials: ChannelCredentials.insecure()),
    );
  }
  final server = serverKey(host, port);
  return GrpcOrGrpcWebClientChannel.grpc(
    host,
    port: port,
    options: ChannelOptions(
      credentials: ChannelCredentials.secure(
        onBadCertificate: (X509Certificate c, String _) => store.accept(
          server,
          CertInfo.fromDer(
            c.der,
            subject: c.subject,
            issuer: c.issuer,
            notBefore: c.startValidity,
            notAfter: c.endValidity,
          ),
        ),
      ),
    ),
  );
}
