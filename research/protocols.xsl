<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
  <xsl:output method="html" encoding="UTF-8" indent="yes" />

  <xsl:template match="/">
    <html>
      <head>
        <meta charset="utf-8" />
        <title>Protocols Build <xsl:value-of select="Protocols/@BuildId"/></title>
      </head>

      <body style="margin:0;padding:20px;font-family:system-ui,-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;font-size:14px;line-height:1.4;background:#121212;color:#e8e8e8;">
        <h1 style="margin:0 0 18px;font-size:22px;line-height:1.2;font-weight:700;color:#f5f5f5;">
          Protocols
          <span style="margin-left:6px;font-size:13px;font-weight:500;color:#8c8c8c;">
            Build <xsl:value-of select="Protocols/@BuildId" />
          </span>
        </h1>

        <xsl:apply-templates select="Protocols/Protocol" />
      </body>
    </html>
  </xsl:template>

  <xsl:template match="Protocol">
    <details style="margin:0 0 14px;">
      <summary style="cursor:pointer;padding:6px 0;font-size:17px;font-weight:700;color:#f0f0f0;border-bottom:1px solid #333;">
        <xsl:value-of select="@Name" />
      </summary>

      <div style="margin-left:12px;">
        <xsl:apply-templates select="Messages" />
      </div>
    </details>
  </xsl:template>

  <xsl:template match="Messages">
    <details style="margin:10px 0;">
      <summary style="cursor:pointer;padding:3px 0;font-size:15px;font-weight:600;color:#d8d8d8;">
        <xsl:value-of select="@Name" />
      </summary>

      <div style="margin-left:12px;">
        <xsl:apply-templates select="Client|Server" />
      </div>
    </details>
  </xsl:template>

  <xsl:template match="Client|Server">
    <details style="margin:8px 0;">
      <summary style="cursor:pointer;padding:3px 0;font-size:14px;font-weight:600;color:#c8c8c8;">
        <xsl:value-of select="name()" />
      </summary>

      <div style="margin-left:12px;">
        <xsl:apply-templates select="Message" />
      </div>
    </details>
  </xsl:template>

  <xsl:template match="Message">
    <div style="margin:6px 0;border:1px solid #343434;border-radius:5px;background:#1b1b1b;overflow:hidden;">
      <div style="display:flex;align-items:center;gap:8px;padding:6px 9px;background:#222;color:#e8e8e8;">
        <span style="font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace;font-size:12px;font-weight:600;color:#d2a8ff;">
          #<xsl:value-of select="@Id" />
        </span>

        <strong style="font-size:13px;font-weight:600;">
          <xsl:value-of select="@Name" />
        </strong>
      </div>

      <xsl:choose>
        <xsl:when test="*">
          <div style="padding:5px 8px 7px;font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace;font-size:12px;">
            <xsl:apply-templates select="*" mode="field" />
          </div>
        </xsl:when>

        <xsl:otherwise>
          <div style="padding:6px 9px;font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace;font-size:12px;font-style:italic;color:#777;">
            no fields
          </div>
        </xsl:otherwise>
      </xsl:choose>
    </div>
  </xsl:template>

  <xsl:template match="*" mode="field">
    <xsl:choose>
      <xsl:when test="*">
        <div style="margin:0;">
          <div style="display:flex;align-items:center;gap:6px;min-height:21px;padding:1px 4px;">
            <xsl:call-template name="field" />
          </div>

          <div style="margin-left:9px;padding-left:11px;border-left:1px solid #383838;">
            <xsl:apply-templates select="*" mode="field" />
          </div>
        </div>
      </xsl:when>

      <xsl:otherwise>
        <div style="display:flex;align-items:center;gap:6px;min-height:21px;padding:1px 4px;">
          <xsl:call-template name="field" />
        </div>
      </xsl:otherwise>
    </xsl:choose>
  </xsl:template>

  <xsl:template name="field">
    <span style="font-weight:600;color:#d2a8ff;">
      <xsl:value-of select="name()" />
    </span>

    <xsl:if test="@TypeName">
      <span style="color:#888;">
        &lt;<xsl:value-of select="@TypeName" />&gt;
      </span>
    </xsl:if>

    <xsl:if test="@Name">
      <span style="color:#dedede;">
        <xsl:value-of select="@Name" />
      </span>
    </xsl:if>

    <xsl:if test="@Size">
      <span style="color:#888;">
        [<xsl:value-of select="@Size" />]
      </span>
    </xsl:if>
  </xsl:template>
</xsl:stylesheet>
