import { apiFetch } from "$lib/api";
import type { FollowUpItem } from "$lib/types";

export function editFollowUp(item: FollowUpItem, question: string, answer: string) {
  return apiFetch<FollowUpItem>(`/follow-ups/${item.id}`, {
    method: "PUT", headers: { "Content-Type": "application/json" },
    body: JSON.stringify({question, answer, expected_question: item.question, expected_answer: item.answer}),
  });
}

export function deleteFollowUp(id: number) {
  return apiFetch(`/follow-ups/${id}`, {method: "DELETE"});
}
