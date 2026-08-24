package ttagy

import (
	"encoding/json"
	"errors"
	"regexp"
	"strings"
)

var mdCodeBlockRegex = regexp.MustCompile("(?s)```(?:json)?\\s*([\\s\\S]*?)\\s*```")

// ExtractStructuredJSON 从可能包含 Markdown 围栏或文本杂音中精准提取最外层有效 JSON
func ExtractStructuredJSON(raw string) (string, error) {
	trimmed := strings.TrimSpace(raw)

	// 1. 直接尝试
	if json.Valid([]byte(trimmed)) {
		return trimmed, nil
	}

	// 2. 剥离 Markdown 围栏
	if matches := mdCodeBlockRegex.FindStringSubmatch(trimmed); len(matches) > 1 {
		candidate := strings.TrimSpace(matches[1])
		if json.Valid([]byte(candidate)) {
			return candidate, nil
		}
		trimmed = candidate
	}

	// 3. 状态机扫描平衡括号
	inString := false
	escape := false
	depth := 0
	startIndex := -1

	runes := []rune(trimmed)
	for i, c := range runes {
		if escape {
			escape = false
			continue
		}
		if c == '\\' {
			escape = true
			continue
		}
		if c == '"' {
			inString = !inString
			continue
		}
		if inString {
			continue
		}

		if c == '{' || c == '[' {
			if depth == 0 {
				startIndex = i
			}
			depth++
		} else if c == '}' || c == ']' {
			if depth > 0 {
				depth--
				if depth == 0 && startIndex != -1 {
					candidate := string(runes[startIndex : i+1])
					if json.Valid([]byte(candidate)) {
						return candidate, nil
					}
				}
			}
		}
	}

	return "", errors.New("failed to extract balanced valid JSON from response")
}
