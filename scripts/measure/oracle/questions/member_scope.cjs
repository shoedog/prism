'use strict';

const memberPathReason = 'member-path-not-judged';

// Match the name token at the claimed bytes. An expression inside a computed key
// and a local binding in a shorthand or destructuring element have value roles.
function memberEndpoint(ts, checker, endpoint, node) {
  if (endpoint.path.fields.length) return true;
  if (!node?.parent || ts.isComputedPropertyName(node)) return false;
  if (ts.isPrivateIdentifier(node)) return true;

  const parent = node.parent;
  if (ts.isQualifiedName(parent) && parent.right === node) return true;
  if (ts.isBindingElement(parent) && parent.propertyName === node) return true;
  if (ts.isJsxNamespacedName(parent) && ts.isJsxAttribute(parent.parent)) return true;
  if (parent.name === node && (ts.isPropertyAccessExpression(parent) || ts.isPropertyAssignment(parent)
      || ts.isPropertyDeclaration(parent) || ts.isPropertySignature(parent)
      || ts.isMethodDeclaration(parent) || ts.isMethodSignature(parent)
      || ts.isGetAccessorDeclaration(parent) || ts.isSetAccessorDeclaration(parent)
      || ts.isJsxAttribute(parent) || ts.isEnumMember(parent)
      || (ts.isMetaProperty(parent) && parent.keywordToken === ts.SyntaxKind.NewKeyword))) return true;

  return ts.isIdentifier(node)
    && (checker.getSymbolAtLocation(node)?.flags & ts.SymbolFlags.EnumMember) !== 0;
}

module.exports = { memberEndpoint, memberPathReason };
