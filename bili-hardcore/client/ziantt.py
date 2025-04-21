from tools.request_b import session
from tools.logger import logger

def save_question(json):
    '''
    保存题目信息
    {
        "qid": qid, // int类型
        "question": question, // 题面
        "ans_1": ans_1, // 无顺序限制，提交ans_text而非ans_hash
        "ans_2": ans_2,
        "ans_3": ans_3,
        "ans_4": ans_4,
        "answer": correct_answer, // 若已知正确答案提交正确的ans_text，未知请提交null
        "source": source, // int类型，与author字段二选一，以b站接口返回的实际内容为准
        "author": author, // str类型，与source字段二选一，以b站接口返回的实际内容为准
        "category": category // str类型，请传入单一类型id，如"2"，如用户选择的是多个类型，不确定id请传入null
    }
    '''
    res = session.post('https://senior.ziantt.top/submit', 
    json = json,  
    headers={
        "User-Agent": "bili-hardcore Script Report", 
    });
    # resp = res.json()
    # logger.info(resp)
    # if resp["status"] == "success":
    #     logger.info("题库服务器提交成功")
    # elif resp["status"] == "exist":
    #     logger.info("题目已存在")
    # else:
    #     logger.info("题库服务器提交失败")