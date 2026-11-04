// v1.1.0 certificate warnings and the trusted-certificates list.
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'cert_trust.dart';

Widget _facts(BuildContext context, CertInfo c) => Column(
  crossAxisAlignment: CrossAxisAlignment.start,
  children: [
    const Text('SHA-256 fingerprint:'),
    SelectableText(
      c.grouped,
      key: const Key('cert-fingerprint'),
      style: const TextStyle(fontFamily: 'monospace'),
    ),
    const SizedBox(height: 8),
    Text('Subject: ${c.subject.isEmpty ? '?' : c.subject}'),
    Text('Issuer: ${c.issuer.isEmpty ? '?' : c.issuer}'),
    Text('Expires: ${fmtDate(c.notAfter)}'),
  ],
);

/// Shows the warning for [p]; returns true when the user trusted the certificate (already saved
/// in [store]).
Future<bool> showCertWarning(
  BuildContext context,
  CertProblem p,
  TrustStore store, {
  DateTime? now,
}) async {
  final t = now ?? DateTime.now();
  final changed = p.verdict == CertVerdict.changed;
  final override = p.cert.needsOverrideAt(t);
  var confirmed = false;
  final ok = await showDialog<bool>(
    context: context,
    barrierDismissible: false,
    builder: (ctx) => StatefulBuilder(
      builder: (ctx, set) => AlertDialog(
        key: const Key('cert-warning'),
        title: Text(
          changed
              ? 'WARNING: the certificate of ${p.server} has changed'
              : 'Certificate not trusted: ${p.server}',
          style: changed ? const TextStyle(color: Colors.red) : null,
        ),
        content: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                changed
                    ? 'This server now presents a different certificate than the one you '
                          'trusted. Someone may be intercepting the connection. The app will '
                          'not connect unless you check the new fingerprint with the server\'s '
                          'administrator and replace the trusted certificate.'
                    : 'This device does not trust the certificate (self-signed, a private CA '
                          'or the wrong name). Compare the fingerprint with the one shown on the '
                          'server before trusting it.',
              ),
              if (changed && p.previous != null) ...[
                const SizedBox(height: 8),
                Text('Trusted before: ${p.previous!.fingerprint}'),
              ],
              if (override) ...[
                const SizedBox(height: 8),
                Text(
                  p.cert.expiredAt(t)
                      ? 'This certificate EXPIRED on ${fmtDate(p.cert.notAfter)}.'
                      : 'This certificate is not valid yet.',
                  key: const Key('cert-expired'),
                  style: const TextStyle(color: Colors.red),
                ),
              ],
              const SizedBox(height: 12),
              _facts(ctx, p.cert),
              if (changed || override)
                CheckboxListTile(
                  key: const Key('cert-confirm'),
                  contentPadding: EdgeInsets.zero,
                  value: confirmed,
                  onChanged: (v) => set(() => confirmed = v ?? false),
                  title: Text(
                    changed
                        ? 'I checked the new fingerprint with the administrator'
                        : 'I understand the risk; trust it anyway',
                  ),
                ),
            ],
          ),
        ),
        actions: [
          TextButton(
            key: const Key('cert-copy'),
            onPressed: () =>
                Clipboard.setData(ClipboardData(text: p.cert.fingerprint)),
            child: const Text('Copy fingerprint'),
          ),
          TextButton(
            key: const Key('cert-cancel'),
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: Key(changed ? 'cert-replace' : 'cert-trust'),
            onPressed: (changed || override) && !confirmed
                ? null
                : () => Navigator.pop(ctx, true),
            child: Text(
              changed
                  ? 'Replace trusted certificate'
                  : override
                  ? 'Trust anyway'
                  : 'Trust this certificate',
            ),
          ),
        ],
      ),
    ),
  );
  if (ok == true) {
    store.trust(p.server, p.cert, now: t);
    await store.save();
    return true;
  }
  return false;
}

/// Settings: the certificates this device trusts, with remove.
class TrustedCertsSection extends StatefulWidget {
  final TrustStore store;
  const TrustedCertsSection({super.key, required this.store});
  @override
  State<TrustedCertsSection> createState() => _TrustedCertsSectionState();
}

class _TrustedCertsSectionState extends State<TrustedCertsSection> {
  @override
  Widget build(BuildContext context) {
    final all = widget.store.all;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Trusted certificates',
          style: Theme.of(context).textTheme.titleMedium,
        ),
        if (all.isEmpty)
          const Text(
            'None: every server is checked against this system\'s trust store.',
            key: Key('certs-none'),
          ),
        for (final t in all)
          ListTile(
            key: Key('cert-${t.server}'),
            dense: true,
            title: Text(t.server),
            subtitle: Text(
              '${t.fingerprint}\n${t.subject}  expires ${t.notAfter}',
            ),
            isThreeLine: true,
            trailing: IconButton(
              key: Key('cert-remove-${t.server}'),
              tooltip: 'Remove (ask again next time)',
              icon: const Icon(Icons.delete),
              onPressed: () async {
                widget.store.remove(t.server);
                await widget.store.save();
                if (mounted) setState(() {});
              },
            ),
          ),
      ],
    );
  }
}
