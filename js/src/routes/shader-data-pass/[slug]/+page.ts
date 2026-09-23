import { error } from "@sveltejs/kit";
import type { PageLoad } from "./$types";

const knownSlugs = ["inter-stage"]; // 可换成从 CMS/数据库动态查询

export const load: PageLoad = ({ params }) => {
  if (!knownSlugs.includes(params.slug)) {
    error(404, "页面不存在");
  }
  return { slug: params.slug };
};
